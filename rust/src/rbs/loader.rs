use crate::env::GlobalEnv;
use crate::rbs::converter::RbsTypeConverter;
use crate::rbs::error::RbsError;
use crate::types::Type;
use magnus::value::ReprValue;
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};

/// Method information loaded from RBS
#[derive(Debug, Clone)]
pub struct RbsMethodInfo {
    pub receiver_class: String,
    pub method_name: String,
    pub return_type: Type,
    pub block_param_types: Option<Vec<String>>,
}

/// Loader that calls RBS API via magnus to load method information
pub struct RbsLoader<'a> {
    ruby: &'a Ruby,
}

impl<'a> RbsLoader<'a> {
    /// Create a new RbsLoader
    /// Assumes RBS gem is already loaded on the Ruby side
    pub fn new(ruby: &'a Ruby) -> Result<Self, RbsError> {
        // Check if RBS module is defined
        let rbs_defined: bool = ruby
            .eval("defined?(RBS) == 'constant'")
            .map_err(|e| RbsError::LoadError(format!("Failed to check RBS: {}", e)))?;

        if !rbs_defined {
            return Err(RbsError::RbsNotInstalled);
        }

        Ok(Self { ruby })
    }

    /// Load all method definitions from RBS
    pub fn load_methods(&self) -> Result<Vec<RbsMethodInfo>, RbsError> {
        // Load method_loader.rb
        let rb_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/rbs/method_loader.rb");
        let load_code = format!("require '{}'", rb_path);
        let _: Value = self
            .ruby
            .eval(&load_code)
            .map_err(|e| RbsError::LoadError(format!("Failed to load method_loader.rb: {}", e)))?;

        // Instantiate Rbs::MethodLoader class and call method
        let results: Value = self
            .ruby
            .eval("Rbs::MethodLoader.new.load_methods")
            .map_err(|e| {
                RbsError::LoadError(format!(
                    "Failed to call Rbs::MethodLoader#load_methods: {}",
                    e
                ))
            })?;

        self.parse_results(results)
    }

    /// Convert Ruby array results to Vec of RbsMethodInfo structs
    fn parse_results(&self, results: Value) -> Result<Vec<RbsMethodInfo>, RbsError> {
        let mut method_infos = Vec::new();

        // Convert to Ruby array
        let results_array = RArray::try_convert(results)
            .map_err(|e| RbsError::ParseError(format!("Failed to convert to array: {}", e)))?;

        // Process each element
        for entry in results_array.into_iter() {
            let entry: Value = entry;

            // Convert to hash
            let hash = RHash::try_convert(entry).map_err(|e| {
                RbsError::ParseError(format!("Failed to convert entry to hash: {}", e))
            })?;

            // Get each field
            let receiver_class_value = hash
                .get(self.ruby.to_symbol("receiver_class"))
                .ok_or_else(|| RbsError::ParseError("Missing receiver_class".to_string()))?;
            let receiver_class: String =
                String::try_convert(receiver_class_value).map_err(|e| {
                    RbsError::ParseError(format!("Failed to convert receiver_class: {}", e))
                })?;

            let method_name_value = hash
                .get(self.ruby.to_symbol("method_name"))
                .ok_or_else(|| RbsError::ParseError("Missing method_name".to_string()))?;
            let method_name: String = String::try_convert(method_name_value).map_err(|e| {
                RbsError::ParseError(format!("Failed to convert method_name: {}", e))
            })?;

            let return_type_value = hash
                .get(self.ruby.to_symbol("return_type"))
                .ok_or_else(|| RbsError::ParseError("Missing return_type".to_string()))?;
            let return_type_str: String = String::try_convert(return_type_value).map_err(|e| {
                RbsError::ParseError(format!("Failed to convert return_type: {}", e))
            })?;

            // Convert RBS type string to internal Type enum
            let return_type = RbsTypeConverter::parse(&return_type_str);

            // Parse block_param_types (optional)
            let block_param_types: Option<Vec<String>> =
                if let Some(bpt_value) = hash.get(self.ruby.to_symbol("block_param_types")) {
                    if bpt_value.is_nil() {
                        None
                    } else if let Ok(bpt_array) = RArray::try_convert(bpt_value) {
                        let types: Vec<String> = bpt_array
                            .into_iter()
                            .filter_map(|v| String::try_convert(v).ok())
                            .collect();
                        if types.is_empty() {
                            None
                        } else {
                            Some(types)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

            method_infos.push(RbsMethodInfo {
                receiver_class,
                method_name,
                return_type,
                block_param_types,
            });
        }

        Ok(method_infos)
    }
}

/// Helper function to register RBS methods to GlobalEnv
/// Uses cache to avoid slow Ruby FFI calls
pub fn register_rbs_methods(genv: &mut GlobalEnv, ruby: &Ruby) -> Result<usize, Error> {
    use crate::cache::RbsCache;

    let methodray_version = env!("CARGO_PKG_VERSION");

    // Try to get RBS version
    let rbs_version_value: Value = ruby
        .eval("RBS::VERSION")
        .unwrap_or_else(|_| ruby.eval("'unknown'").unwrap());
    let rbs_version: String =
        String::try_convert(rbs_version_value).unwrap_or_else(|_| "unknown".to_string());

    // Try to load from cache
    let methods = if let Ok(cache) = RbsCache::load() {
        if cache.is_valid(methodray_version, &rbs_version) {
            cache.to_method_infos()
        } else {
            eprintln!("Cache invalid, reloading from RBS...");
            let methods = load_and_cache_rbs_methods(ruby, methodray_version, &rbs_version)?;
            methods
        }
    } else {
        eprintln!("No cache found, loading from RBS...");
        load_and_cache_rbs_methods(ruby, methodray_version, &rbs_version)?
    };

    let count = methods.len();
    for method_info in methods {
        let receiver_type = Type::instance(&method_info.receiver_class);
        // Convert block param type strings to Type enums
        let block_param_types = method_info.block_param_types.map(|types| {
            types
                .iter()
                .map(|s| RbsTypeConverter::parse(s))
                .collect()
        });
        genv.register_builtin_method_with_block(
            receiver_type,
            &method_info.method_name,
            method_info.return_type,
            block_param_types,
        );
    }

    Ok(count)
}

/// Load RBS methods and save to cache
fn load_and_cache_rbs_methods(
    ruby: &Ruby,
    version: &str,
    rbs_version: &str,
) -> Result<Vec<RbsMethodInfo>, Error> {
    use crate::cache::RbsCache;

    let loader = RbsLoader::new(ruby)?;
    let methods = loader.load_methods()?;

    // Save to cache
    let cache = RbsCache::from_method_infos(
        methods.clone(),
        version.to_string(),
        rbs_version.to_string(),
    );

    if let Err(e) = cache.save() {
        eprintln!("Warning: Failed to save RBS cache: {}", e);
    } else {
        eprintln!("Saved {} methods to cache", methods.len());
    }

    Ok(methods)
}
