use crate::types::Type;

// RBS Type Converter
pub struct RbsTypeConverter;

impl RbsTypeConverter {
    pub fn parse(rbs_type: &str) -> Type {
        // Handle union types
        if rbs_type.contains(" | ") {
            let parts: Vec<&str> = rbs_type.split(" | ").collect();
            let types: Vec<Type> = parts.iter().map(|s| Self::parse_single(s.trim())).collect();
            return Type::Union(types);
        }

        Self::parse_single(rbs_type)
    }

    fn parse_single(rbs_type: &str) -> Type {
        let type_name = rbs_type.trim_start_matches("::");

        match type_name {
            "bool" => Type::Union(vec![
                Type::instance("TrueClass"),
                Type::instance("FalseClass"),
            ]),
            "void" | "nil" => Type::Nil,
            "untyped" | "top" => Type::Bot,
            _ => Type::instance(type_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_types() {
        match RbsTypeConverter::parse("::String") {
            Type::Instance { name } => assert_eq!(name.full_name(), "String"),
            _ => panic!("Expected Instance type"),
        }

        match RbsTypeConverter::parse("Integer") {
            Type::Instance { name } => assert_eq!(name.full_name(), "Integer"),
            _ => panic!("Expected Instance type"),
        }
    }

    #[test]
    fn test_parse_qualified_types() {
        match RbsTypeConverter::parse("::Api::User") {
            Type::Instance { name } => {
                assert_eq!(name.full_name(), "Api::User");
                assert_eq!(name.name(), "User");
            }
            _ => panic!("Expected Instance type"),
        }
    }

    #[test]
    fn test_parse_special_types() {
        assert!(matches!(RbsTypeConverter::parse("nil"), Type::Nil));
        assert!(matches!(RbsTypeConverter::parse("void"), Type::Nil));
        assert!(matches!(RbsTypeConverter::parse("untyped"), Type::Bot));
    }

    #[test]
    fn test_parse_bool() {
        match RbsTypeConverter::parse("bool") {
            Type::Union(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Union type for bool"),
        }
    }

    #[test]
    fn test_parse_union_types() {
        match RbsTypeConverter::parse("String | Integer") {
            Type::Union(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Union type"),
        }
    }
}
