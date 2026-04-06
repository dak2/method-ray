use crate::types::Type;

// RBS Type Converter
pub struct RbsTypeConverter;

/// Split type arguments by comma, respecting bracket nesting depth.
/// e.g., "String, Array[Integer]" → ["String", "Array[Integer]"]
fn split_type_args(s: &str) -> Vec<&str> {
    let mut results = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        match b {
            b'[' => depth += 1,
            b']' => depth -= 1,
            b',' if depth == 0 => {
                results.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    results.push(s[start..].trim());
    results
}

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
            _ => {
                // Handle generic types: Array[Elem], Hash[K, V]
                // Only parse as generic when base class name is non-empty (skip tuple-like `[...]`)
                if let Some(bracket_start) = type_name.find('[') {
                    if bracket_start > 0 && type_name.ends_with(']') {
                        let base = &type_name[..bracket_start];
                        let args_str = &type_name[bracket_start + 1..type_name.len() - 1];
                        let type_args: Vec<Type> = split_type_args(args_str)
                            .into_iter()
                            .map(Self::parse)
                            .collect();
                        return Type::Generic {
                            name: crate::types::QualifiedName::from(base),
                            type_args,
                        };
                    }
                }
                Type::instance(type_name)
            }
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

    #[test]
    fn test_parse_generic_single_arg() {
        match RbsTypeConverter::parse("Array[Elem]") {
            Type::Generic { name, type_args } => {
                assert_eq!(name.full_name(), "Array");
                assert_eq!(type_args.len(), 1);
                match &type_args[0] {
                    Type::Instance { name } => assert_eq!(name.full_name(), "Elem"),
                    _ => panic!("Expected Instance for type arg"),
                }
            }
            _ => panic!("Expected Generic type"),
        }
    }

    #[test]
    fn test_parse_generic_multiple_args() {
        match RbsTypeConverter::parse("Hash[K, V]") {
            Type::Generic { name, type_args } => {
                assert_eq!(name.full_name(), "Hash");
                assert_eq!(type_args.len(), 2);
            }
            _ => panic!("Expected Generic type"),
        }
    }

    #[test]
    fn test_parse_nested_generic() {
        match RbsTypeConverter::parse("Hash[String, Array[Integer]]") {
            Type::Generic { name, type_args } => {
                assert_eq!(name.full_name(), "Hash");
                assert_eq!(type_args.len(), 2);
                match &type_args[0] {
                    Type::Instance { name } => assert_eq!(name.full_name(), "String"),
                    _ => panic!("Expected Instance for first arg"),
                }
                match &type_args[1] {
                    Type::Generic { name, type_args } => {
                        assert_eq!(name.full_name(), "Array");
                        assert_eq!(type_args.len(), 1);
                    }
                    _ => panic!("Expected Generic for second arg"),
                }
            }
            _ => panic!("Expected Generic type"),
        }
    }

    #[test]
    fn test_parse_bare_bracket_not_generic() {
        // "[String]" should not be parsed as generic (bracket_start == 0)
        match RbsTypeConverter::parse("[String]") {
            Type::Instance { name } => assert_eq!(name.full_name(), "[String]"),
            _ => panic!("Expected Instance type for bare bracket"),
        }
    }

    #[test]
    fn test_split_type_args_nested() {
        let result = split_type_args("String, Array[Integer]");
        assert_eq!(result, vec!["String", "Array[Integer]"]);

        let result = split_type_args("Array[K, V], Integer");
        assert_eq!(result, vec!["Array[K, V]", "Integer"]);
    }
}
