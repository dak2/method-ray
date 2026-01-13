use std::collections::HashMap;

/// グラフベース型推論用
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// インスタンス型: String, Integer等
    Instance { class_name: String },
    /// シングルトン型: クラスメソッド用
    Singleton { class_name: String },
    /// nil型
    Nil,
    /// Union型: 複数の型の和
    Union(Vec<Type>),
    /// ボトム型: 型情報がない
    Bot,
}

impl Type {
    /// 型を文字列表現に変換
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

    /// 便利なコンストラクタ
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

/// 後方互換性のためにRubyTypeも残す（徐々にTypeに移行）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RubyType {
    String,
    Integer,
    Float,
    Array,
    Hash,
    Symbol,
    TrueClass,
    FalseClass,
    NilClass,
    Custom(String),
    Unknown,
}

impl RubyType {
    pub fn to_class_name(&self) -> &str {
        match self {
            RubyType::String => "String",
            RubyType::Integer => "Integer",
            RubyType::Float => "Float",
            RubyType::Array => "Array",
            RubyType::Hash => "Hash",
            RubyType::Symbol => "Symbol",
            RubyType::TrueClass => "TrueClass",
            RubyType::FalseClass => "FalseClass",
            RubyType::NilClass => "NilClass",
            RubyType::Custom(name) => name,
            RubyType::Unknown => "Unknown",
        }
    }

    /// RubyType から Type への変換
    pub fn to_type(&self) -> Type {
        match self {
            RubyType::String => Type::string(),
            RubyType::Integer => Type::integer(),
            RubyType::Float => Type::Instance {
                class_name: "Float".to_string(),
            },
            RubyType::Array => Type::array(),
            RubyType::Hash => Type::hash(),
            RubyType::Symbol => Type::Instance {
                class_name: "Symbol".to_string(),
            },
            RubyType::TrueClass => Type::Instance {
                class_name: "TrueClass".to_string(),
            },
            RubyType::FalseClass => Type::Instance {
                class_name: "FalseClass".to_string(),
            },
            RubyType::NilClass => Type::Nil,
            RubyType::Custom(name) => Type::Instance {
                class_name: name.clone(),
            },
            RubyType::Unknown => Type::Bot,
        }
    }
}

/// 変数の型情報
#[derive(Debug, Clone)]
pub struct VariableType {
    pub name: String,
    pub ruby_type: RubyType,
    pub location: Location,
}

/// ソースコード位置
#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// メソッド呼び出し情報
#[derive(Debug, Clone)]
pub struct MethodCall {
    pub receiver_type: RubyType,
    pub method_name: String,
    pub location: Location,
    pub is_defined: bool,
}

/// 解析結果
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub file_path: String,
    pub method_calls: Vec<MethodCall>,
    pub undefined_methods: Vec<MethodCall>,
}

/// ローカル変数のコンテキスト（Phase 2で使用）
#[derive(Debug, Clone)]
pub struct LocalContext {
    variables: HashMap<String, VariableType>,
}

impl LocalContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set_variable(&mut self, name: String, ruby_type: RubyType, location: Location) {
        self.variables.insert(
            name.clone(),
            VariableType {
                name,
                ruby_type,
                location,
            },
        );
    }

    pub fn get_variable(&self, name: &str) -> Option<&VariableType> {
        self.variables.get(name)
    }

    pub fn clear(&mut self) {
        self.variables.clear();
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

    #[test]
    fn test_ruby_type_to_type() {
        assert_eq!(RubyType::String.to_type(), Type::string());
        assert_eq!(RubyType::Integer.to_type(), Type::integer());
        assert_eq!(RubyType::NilClass.to_type(), Type::Nil);
    }

    #[test]
    fn test_ruby_type_to_class_name() {
        assert_eq!(RubyType::String.to_class_name(), "String");
        assert_eq!(RubyType::Integer.to_class_name(), "Integer");
        assert_eq!(RubyType::Array.to_class_name(), "Array");
        assert_eq!(RubyType::Custom("User".to_string()).to_class_name(), "User");
    }

    #[test]
    fn test_local_context() {
        let mut context = LocalContext::new();
        let location = Location {
            file: "test.rb".to_string(),
            line: 1,
            column: 0,
        };

        context.set_variable("x".to_string(), RubyType::String, location.clone());

        let var = context.get_variable("x").unwrap();
        assert_eq!(var.name, "x");
        assert_eq!(var.ruby_type, RubyType::String);
    }
}
