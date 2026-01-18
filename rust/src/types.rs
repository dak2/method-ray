/// Type system for graph-based type inference
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Type {
    /// Instance type: String, Integer, etc.
    Instance { class_name: String },
    /// Singleton type: for class methods
    Singleton { class_name: String },
    /// nil type
    Nil,
    /// Union type: sum of multiple types
    Union(Vec<Type>),
    /// Bottom type: no type information
    Bot,
}

impl Type {
    /// Convert type to string representation
    pub fn show(&self) -> String {
        match self {
            Type::Instance { class_name } => class_name.clone(),
            Type::Singleton { class_name } => format!("singleton({})", class_name),
            Type::Nil => "nil".to_string(),
            Type::Union(types) => {
                let names: Vec<_> = types.iter().map(|t| t.show()).collect();
                names.join(" | ")
            }
            Type::Bot => "untyped".to_string(),
        }
    }

    /// Convenience constructors
    pub fn string() -> Self {
        Type::Instance {
            class_name: "String".to_string(),
        }
    }

    pub fn integer() -> Self {
        Type::Instance {
            class_name: "Integer".to_string(),
        }
    }

    pub fn array() -> Self {
        Type::Instance {
            class_name: "Array".to_string(),
        }
    }

    pub fn hash() -> Self {
        Type::Instance {
            class_name: "Hash".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_show() {
        assert_eq!(Type::string().show(), "String");
        assert_eq!(Type::integer().show(), "Integer");
        assert_eq!(Type::Nil.show(), "nil");
        assert_eq!(Type::Bot.show(), "untyped");
    }

    #[test]
    fn test_type_union() {
        let union = Type::Union(vec![Type::string(), Type::integer()]);
        assert_eq!(union.show(), "String | Integer");
    }
}
