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
    /// Get cache file path
    pub fn cache_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to get cache directory")?
            .join("methodray");

        fs::create_dir_all(&cache_dir)
            .context("Failed to create cache directory")?;

        Ok(cache_dir.join("rbs_cache.bin"))
    }

    /// Load cache from disk
    pub fn load() -> Result<Self> {
        let path = Self::cache_path()?;
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read cache from {}", path.display()))?;

        bincode::deserialize(&bytes)
            .context("Failed to deserialize cache")
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::cache_path()?;
        let bytes = bincode::serialize(self)
            .context("Failed to serialize cache")?;

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
}
