use smallvec::SmallVec;

/// Qualified name for classes and modules (e.g., "Api::V1::User")
/// Uses compact representation: stores full name as single String with segment offsets
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    /// Full qualified name (e.g., "Api::V1::User")
    full: String,
    /// Start offset of each segment (e.g., [0, 5, 10] for "Api::V1::User")
    /// Up to 16 segments stored inline on stack
    offsets: SmallVec<[u16; 16]>,
}

impl QualifiedName {
    /// Create a new QualifiedName from a full qualified name string
    pub fn new(full: &str) -> Self {
        let mut offsets = SmallVec::new();
        offsets.push(0);
        for (i, _) in full.match_indices("::") {
            offsets.push((i + 2) as u16);
        }
        Self {
            full: full.to_string(),
            offsets,
        }
    }

    /// Create a QualifiedName for a simple (non-namespaced) name
    pub fn simple(name: &str) -> Self {
        Self {
            full: name.to_string(),
            offsets: smallvec::smallvec![0],
        }
    }

    /// Get the last segment (class/module name without namespace)
    pub fn name(&self) -> &str {
        let start = *self.offsets.last().unwrap() as usize;
        &self.full[start..]
    }

    /// Get the full qualified name string
    pub fn full_name(&self) -> &str {
        &self.full
    }

    /// Get the number of segments
    pub fn depth(&self) -> usize {
        self.offsets.len()
    }

    /// Check if this is a simple (non-namespaced) name
    pub fn is_simple(&self) -> bool {
        self.offsets.len() == 1
    }

    /// Get the n-th segment (0-indexed)
    pub fn segment(&self, n: usize) -> Option<&str> {
        if n >= self.offsets.len() {
            return None;
        }
        let start = self.offsets[n] as usize;
        let end = self
            .offsets
            .get(n + 1)
            .map(|&o| o as usize - 2) // subtract "::"
            .unwrap_or(self.full.len());
        Some(&self.full[start..end])
    }

    /// Get the parent namespace (e.g., "Api::V1" for "Api::V1::User")
    pub fn parent(&self) -> Option<Self> {
        if self.offsets.len() <= 1 {
            return None;
        }
        let last_offset = self.offsets[self.offsets.len() - 1] as usize;
        let parent_full = &self.full[..last_offset - 2]; // exclude "::"
        Some(Self {
            full: parent_full.to_string(),
            offsets: self.offsets[..self.offsets.len() - 1].into(),
        })
    }

    /// Create a child by appending a name segment
    pub fn child(&self, name: &str) -> Self {
        let mut full = self.full.clone();
        full.push_str("::");
        full.push_str(name);

        let mut offsets = self.offsets.clone();
        offsets.push((self.full.len() + 2) as u16);

        Self { full, offsets }
    }

    /// Join two qualified names
    pub fn join(&self, other: &QualifiedName) -> Self {
        let mut result = self.clone();
        for i in 0..other.depth() {
            if let Some(seg) = other.segment(i) {
                result = result.child(seg);
            }
        }
        result
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full)
    }
}

impl From<&str> for QualifiedName {
    fn from(s: &str) -> Self {
        QualifiedName::new(s)
    }
}

impl From<String> for QualifiedName {
    fn from(s: String) -> Self {
        QualifiedName::new(&s)
    }
}

/// Type system for graph-based type inference
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Type {
    /// Instance type: String, Integer, Api::User, etc.
    Instance { name: QualifiedName },
    /// Generic instance type: Array[Integer], Hash[String, Integer], etc.
    Generic {
        name: QualifiedName,
        type_args: Vec<Type>,
    },
    /// Singleton type: for class methods
    Singleton { name: QualifiedName },
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
            Type::Instance { name } => name.full_name().to_string(),
            Type::Generic { name, type_args } => {
                let args: Vec<_> = type_args.iter().map(|t| t.show()).collect();
                format!("{}[{}]", name.full_name(), args.join(", "))
            }
            Type::Singleton { name } => format!("singleton({})", name.full_name()),
            Type::Nil => "nil".to_string(),
            Type::Union(types) => {
                let names: Vec<_> = types.iter().map(|t| t.show()).collect();
                names.join(" | ")
            }
            Type::Bot => "untyped".to_string(),
        }
    }

    /// Get the qualified name for this type
    pub fn qualified_name(&self) -> Option<&QualifiedName> {
        match self {
            Type::Instance { name } => Some(name),
            Type::Generic { name, .. } => Some(name),
            Type::Singleton { name } => Some(name),
            _ => None,
        }
    }

    /// Get the base class name (full qualified name, without type arguments)
    pub fn base_class_name(&self) -> Option<&str> {
        self.qualified_name().map(|n| n.full_name())
    }

    /// Get just the simple name (without namespace)
    pub fn simple_name(&self) -> Option<&str> {
        self.qualified_name().map(|n| n.name())
    }

    /// Get type arguments for generic types
    pub fn type_args(&self) -> Option<&[Type]> {
        match self {
            Type::Generic { type_args, .. } => Some(type_args),
            _ => None,
        }
    }

    /// Create an instance type from a qualified name string
    pub fn instance(name: &str) -> Self {
        Type::Instance {
            name: QualifiedName::new(name),
        }
    }

    /// Create a singleton type from a qualified name string
    pub fn singleton(name: &str) -> Self {
        Type::Singleton {
            name: QualifiedName::new(name),
        }
    }

    /// Convenience constructors
    pub fn string() -> Self {
        Type::Instance {
            name: QualifiedName::simple("String"),
        }
    }

    pub fn integer() -> Self {
        Type::Instance {
            name: QualifiedName::simple("Integer"),
        }
    }

    pub fn float() -> Self {
        Type::Instance {
            name: QualifiedName::simple("Float"),
        }
    }

    pub fn symbol() -> Self {
        Type::Instance {
            name: QualifiedName::simple("Symbol"),
        }
    }

    pub fn array() -> Self {
        Type::Instance {
            name: QualifiedName::simple("Array"),
        }
    }

    pub fn hash() -> Self {
        Type::Instance {
            name: QualifiedName::simple("Hash"),
        }
    }

    /// Create a generic Array type: Array[element_type]
    pub fn array_of(element_type: Type) -> Self {
        Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![element_type],
        }
    }

    /// Create a generic Hash type: Hash[key_type, value_type]
    pub fn hash_of(key_type: Type, value_type: Type) -> Self {
        Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![key_type, value_type],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // QualifiedName tests
    #[test]
    fn test_qualified_name_simple() {
        let name = QualifiedName::simple("User");
        assert_eq!(name.name(), "User");
        assert_eq!(name.full_name(), "User");
        assert_eq!(name.depth(), 1);
        assert!(name.is_simple());
        assert!(name.parent().is_none());
    }

    #[test]
    fn test_qualified_name_nested() {
        let name = QualifiedName::new("Api::V1::User");
        assert_eq!(name.name(), "User");
        assert_eq!(name.full_name(), "Api::V1::User");
        assert_eq!(name.depth(), 3);
        assert!(!name.is_simple());

        // Test segments
        assert_eq!(name.segment(0), Some("Api"));
        assert_eq!(name.segment(1), Some("V1"));
        assert_eq!(name.segment(2), Some("User"));
        assert_eq!(name.segment(3), None);
    }

    #[test]
    fn test_qualified_name_parent() {
        let name = QualifiedName::new("Api::V1::User");
        let parent = name.parent().unwrap();
        assert_eq!(parent.full_name(), "Api::V1");
        assert_eq!(parent.name(), "V1");

        let grandparent = parent.parent().unwrap();
        assert_eq!(grandparent.full_name(), "Api");
        assert!(grandparent.parent().is_none());
    }

    #[test]
    fn test_qualified_name_child() {
        let name = QualifiedName::simple("Api");
        let child = name.child("V1");
        assert_eq!(child.full_name(), "Api::V1");

        let grandchild = child.child("User");
        assert_eq!(grandchild.full_name(), "Api::V1::User");
        assert_eq!(grandchild.depth(), 3);
    }

    #[test]
    fn test_qualified_name_display() {
        let name = QualifiedName::new("Api::V1::User");
        assert_eq!(format!("{}", name), "Api::V1::User");
    }

    #[test]
    fn test_qualified_name_from() {
        let name: QualifiedName = "Api::User".into();
        assert_eq!(name.full_name(), "Api::User");

        let name2: QualifiedName = String::from("Module::Class").into();
        assert_eq!(name2.full_name(), "Module::Class");
    }

    // Type tests
    #[test]
    fn test_type_show() {
        assert_eq!(Type::string().show(), "String");
        assert_eq!(Type::integer().show(), "Integer");
        assert_eq!(Type::Nil.show(), "nil");
        assert_eq!(Type::Bot.show(), "untyped");
    }

    #[test]
    fn test_type_instance_qualified() {
        let user_type = Type::instance("Api::V1::User");
        assert_eq!(user_type.show(), "Api::V1::User");
        assert_eq!(user_type.base_class_name(), Some("Api::V1::User"));
        assert_eq!(user_type.simple_name(), Some("User"));
    }

    #[test]
    fn test_type_union() {
        let union = Type::Union(vec![Type::string(), Type::integer()]);
        assert_eq!(union.show(), "String | Integer");
    }

    #[test]
    fn test_generic_type_show() {
        let array_int = Type::array_of(Type::integer());
        assert_eq!(array_int.show(), "Array[Integer]");

        let hash_str_int = Type::hash_of(Type::string(), Type::integer());
        assert_eq!(hash_str_int.show(), "Hash[String, Integer]");
    }

    #[test]
    fn test_base_class_name() {
        assert_eq!(Type::string().base_class_name(), Some("String"));
        assert_eq!(
            Type::array_of(Type::integer()).base_class_name(),
            Some("Array")
        );
        assert_eq!(Type::Nil.base_class_name(), None);
        assert_eq!(Type::Bot.base_class_name(), None);
    }

    #[test]
    fn test_type_args() {
        let array_int = Type::array_of(Type::integer());
        let args = array_int.type_args().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].show(), "Integer");

        // Non-generic types have no type args
        assert!(Type::string().type_args().is_none());
    }

    #[test]
    fn test_singleton_type() {
        let singleton = Type::singleton("Api::User");
        assert_eq!(singleton.show(), "singleton(Api::User)");
        assert_eq!(singleton.base_class_name(), Some("Api::User"));
    }
}
