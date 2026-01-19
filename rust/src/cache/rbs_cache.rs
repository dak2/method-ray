use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(feature = "ruby-ffi")]
use crate::rbs::loader::RbsMethodInfo;

/// Binary cache for RBS method definitions
#[derive(Serialize, Deserialize, Debug)]
pub struct RbsCache {
    /// MethodRay version
    pub version: String,
    /// RBS gem version
    pub rbs_version: String,
    /// Cached method information
    pub methods: Vec<SerializableMethodInfo>,
    /// Cache creation timestamp
    pub timestamp: SystemTime,
}

/// Serializable version of RbsMethodInfo
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableMethodInfo {
    pub receiver_class: String,
    pub method_name: String,
    pub return_type_str: String, // Simplified: store as string
}

impl SerializableMethodInfo {
    /// Parse return type string into Type (simple parser for cached data)
    pub fn return_type(&self) -> crate::types::Type {
        crate::types::Type::Instance {
            class_name: self.return_type_str.clone(),
        }
    }
}

#[allow(dead_code)]
impl RbsCache {
    /// Get user cache file path (in ~/.cache/methodray/)
    pub fn cache_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to get cache directory")?
            .join("methodray");

        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        Ok(cache_dir.join("rbs_cache.bin"))
    }

    /// Get bundled cache path (shipped with gem)
    ///
    /// Gem structure after install:
    ///   lib/methodray/
    ///     methodray-cli      # CLI binary
    ///     methodray.bundle   # FFI extension (macOS) or methodray.so (Linux)
    ///     rbs_cache.bin      # Pre-built cache
    ///
    /// The CLI binary and cache are in the same directory.
    fn bundled_cache_path() -> Option<PathBuf> {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Cache is in the same directory as CLI binary
                let bundled = exe_dir.join("rbs_cache.bin");
                if bundled.exists() {
                    return Some(bundled);
                }
            }
        }
        None
    }

    /// Load cache from disk
    /// Tries bundled cache first, then user cache
    pub fn load() -> Result<Self> {
        // Try bundled cache first (shipped with gem)
        if let Some(bundled_path) = Self::bundled_cache_path() {
            if let Ok(bytes) = fs::read(&bundled_path) {
                if let Ok(cache) = bincode::deserialize::<Self>(&bytes) {
                    eprintln!("Loaded bundled cache from {}", bundled_path.display());
                    return Ok(cache);
                }
            }
        }

        // Fall back to user cache
        let path = Self::cache_path()?;
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read cache from {}", path.display()))?;

        bincode::deserialize(&bytes).context("Failed to deserialize cache")
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::cache_path()?;
        let bytes = bincode::serialize(self).context("Failed to serialize cache")?;

        fs::write(&path, bytes)
            .with_context(|| format!("Failed to write cache to {}", path.display()))?;

        Ok(())
    }

    /// Check if cache is valid
    pub fn is_valid(&self, current_version: &str, current_rbs_version: &str) -> bool {
        self.version == current_version && self.rbs_version == current_rbs_version
    }

    /// Get methods for registration (works without ruby-ffi feature)
    pub fn methods(&self) -> &[SerializableMethodInfo] {
        &self.methods
    }

    /// Convert to RbsMethodInfo (requires ruby-ffi for full type parsing)
    #[cfg(feature = "ruby-ffi")]
    pub fn to_method_infos(&self) -> Vec<RbsMethodInfo> {
        self.methods
            .iter()
            .map(|m| RbsMethodInfo {
                receiver_class: m.receiver_class.clone(),
                method_name: m.method_name.clone(),
                return_type: crate::rbs::converter::RbsTypeConverter::parse(&m.return_type_str),
            })
            .collect()
    }

    /// Create from RbsMethodInfo
    #[cfg(feature = "ruby-ffi")]
    pub fn from_method_infos(
        methods: Vec<RbsMethodInfo>,
        version: String,
        rbs_version: String,
    ) -> Self {
        let serializable_methods = methods
            .into_iter()
            .map(|m| SerializableMethodInfo {
                receiver_class: m.receiver_class,
                method_name: m.method_name,
                return_type_str: m.return_type.show(),
            })
            .collect();

        Self {
            version,
            rbs_version,
            methods: serializable_methods,
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_serialization() {
        let cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![SerializableMethodInfo {
                receiver_class: "String".to_string(),
                method_name: "upcase".to_string(),
                return_type_str: "String".to_string(),
            }],
            timestamp: SystemTime::now(),
        };

        let bytes = bincode::serialize(&cache).unwrap();
        let deserialized: RbsCache = bincode::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.version, "0.1.0");
        assert_eq!(deserialized.methods.len(), 1);
    }

    #[test]
    fn test_cache_validation() {
        let cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![],
            timestamp: SystemTime::now(),
        };

        assert!(cache.is_valid("0.1.0", "3.7.0"));
        assert!(!cache.is_valid("0.2.0", "3.7.0"));
        assert!(!cache.is_valid("0.1.0", "3.8.0"));
    }

    #[test]
    fn test_serializable_method_info_return_type() {
        let method_info = SerializableMethodInfo {
            receiver_class: "String".to_string(),
            method_name: "upcase".to_string(),
            return_type_str: "String".to_string(),
        };

        let return_type = method_info.return_type();
        assert_eq!(return_type.show(), "String");
    }

    #[test]
    fn test_cache_methods_accessor() {
        let cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![
                SerializableMethodInfo {
                    receiver_class: "String".to_string(),
                    method_name: "upcase".to_string(),
                    return_type_str: "String".to_string(),
                },
                SerializableMethodInfo {
                    receiver_class: "Integer".to_string(),
                    method_name: "to_s".to_string(),
                    return_type_str: "String".to_string(),
                },
            ],
            timestamp: SystemTime::now(),
        };

        let methods = cache.methods();
        assert_eq!(methods.len(), 2);
        assert_eq!(methods[0].receiver_class, "String");
        assert_eq!(methods[0].method_name, "upcase");
        assert_eq!(methods[1].receiver_class, "Integer");
        assert_eq!(methods[1].method_name, "to_s");
    }

    #[test]
    fn test_cache_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("test_cache.bin");

        let original_cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![
                SerializableMethodInfo {
                    receiver_class: "String".to_string(),
                    method_name: "upcase".to_string(),
                    return_type_str: "String".to_string(),
                },
                SerializableMethodInfo {
                    receiver_class: "Array".to_string(),
                    method_name: "first".to_string(),
                    return_type_str: "Object".to_string(),
                },
            ],
            timestamp: SystemTime::now(),
        };

        // Save to temp file
        let bytes = bincode::serialize(&original_cache).unwrap();
        fs::write(&cache_path, &bytes).unwrap();

        // Load from temp file
        let loaded_bytes = fs::read(&cache_path).unwrap();
        let loaded_cache: RbsCache = bincode::deserialize(&loaded_bytes).unwrap();

        assert_eq!(loaded_cache.version, "0.1.0");
        assert_eq!(loaded_cache.rbs_version, "3.7.0");
        assert_eq!(loaded_cache.methods.len(), 2);
        assert_eq!(loaded_cache.methods[0].method_name, "upcase");
        assert_eq!(loaded_cache.methods[1].method_name, "first");
    }

    #[test]
    fn test_cache_with_empty_methods() {
        let cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![],
            timestamp: SystemTime::now(),
        };

        let bytes = bincode::serialize(&cache).unwrap();
        let deserialized: RbsCache = bincode::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.methods.len(), 0);
        assert!(deserialized.is_valid("0.1.0", "3.7.0"));
    }

    #[test]
    fn test_cache_validation_version_mismatch() {
        let cache = RbsCache {
            version: "0.1.0".to_string(),
            rbs_version: "3.7.0".to_string(),
            methods: vec![],
            timestamp: SystemTime::now(),
        };

        // Both versions must match
        assert!(!cache.is_valid("0.1.1", "3.7.0"));
        assert!(!cache.is_valid("0.1.0", "3.7.1"));
        assert!(!cache.is_valid("0.2.0", "4.0.0"));
    }
}
