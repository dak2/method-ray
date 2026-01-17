use std::collections::HashMap;
use crate::graph::VertexId;

/// スコープID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// スコープの種類
#[derive(Debug, Clone)]
pub enum ScopeKind {
    TopLevel,
    Class {
        name: String,
        superclass: Option<String>,
    },
    Module {
        name: String,
    },
    Method {
        name: String,
        receiver_type: Option<String>, // レシーバーのクラス名
    },
    Block,
}

/// スコープ情報
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,

    /// ローカル変数
    pub local_vars: HashMap<String, VertexId>,

    /// インスタンス変数（クラス/メソッドスコープのみ）
    pub instance_vars: HashMap<String, VertexId>,

    /// クラス変数（クラススコープのみ）
    pub class_vars: HashMap<String, VertexId>,
}

impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            id,
            kind,
            parent,
            local_vars: HashMap::new(),
            instance_vars: HashMap::new(),
            class_vars: HashMap::new(),
        }
    }

    /// ローカル変数を追加
    pub fn set_local_var(&mut self, name: String, vtx: VertexId) {
        self.local_vars.insert(name, vtx);
    }

    /// ローカル変数を取得
    pub fn get_local_var(&self, name: &str) -> Option<VertexId> {
        self.local_vars.get(name).copied()
    }

    /// インスタンス変数を追加
    pub fn set_instance_var(&mut self, name: String, vtx: VertexId) {
        self.instance_vars.insert(name, vtx);
    }

    /// インスタンス変数を取得
    pub fn get_instance_var(&self, name: &str) -> Option<VertexId> {
        self.instance_vars.get(name).copied()
    }
}

/// スコープマネージャー
#[derive(Debug)]
pub struct ScopeManager {
    scopes: HashMap<ScopeId, Scope>,
    next_id: usize,
    current_scope: ScopeId,
}

impl ScopeManager {
    pub fn new() -> Self {
        let top_level = Scope::new(
            ScopeId(0),
            ScopeKind::TopLevel,
            None,
        );

        let mut scopes = HashMap::new();
        scopes.insert(ScopeId(0), top_level);

        Self {
            scopes,
            next_id: 1,
            current_scope: ScopeId(0),
        }
    }

    /// 新しいスコープを作成
    pub fn new_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.next_id);
        self.next_id += 1;

        let scope = Scope::new(id, kind, Some(self.current_scope));
        self.scopes.insert(id, scope);

        id
    }

    /// スコープに入る
    pub fn enter_scope(&mut self, scope_id: ScopeId) {
        self.current_scope = scope_id;
    }

    /// スコープから出る
    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(parent) = scope.parent {
                self.current_scope = parent;
            }
        }
    }

    /// 現在のスコープを取得
    pub fn current_scope(&self) -> &Scope {
        self.scopes.get(&self.current_scope).unwrap()
    }

    /// 現在のスコープを可変で取得
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.get_mut(&self.current_scope).unwrap()
    }

    /// スコープを取得
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// スコープを可変で取得
    pub fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(&id)
    }

    /// 変数を現在のスコープまたは親スコープから検索
    pub fn lookup_var(&self, name: &str) -> Option<VertexId> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let Some(vtx) = scope.get_local_var(name) {
                    return Some(vtx);
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// インスタンス変数を現在のクラススコープから検索
    pub fn lookup_instance_var(&self, name: &str) -> Option<VertexId> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // クラススコープまで遡る
                match &scope.kind {
                    ScopeKind::Class { .. } => {
                        return scope.get_instance_var(name);
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }

        None
    }

    /// インスタンス変数を現在のクラススコープに設定
    pub fn set_instance_var_in_class(&mut self, name: String, vtx: VertexId) {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // クラススコープを見つけたら設定
                match &scope.kind {
                    ScopeKind::Class { .. } => {
                        if let Some(class_scope) = self.scopes.get_mut(&scope_id) {
                            class_scope.set_instance_var(name, vtx);
                        }
                        return;
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// 現在のクラス名を取得
    pub fn current_class_name(&self) -> Option<String> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let ScopeKind::Class { name, .. } = &scope.kind {
                    return Some(name.clone());
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_manager_creation() {
        let sm = ScopeManager::new();
        assert_eq!(sm.current_scope().id, ScopeId(0));
        assert!(matches!(sm.current_scope().kind, ScopeKind::TopLevel));
    }

    #[test]
    fn test_scope_manager_new_scope() {
        let mut sm = ScopeManager::new();

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });

        assert_eq!(class_id, ScopeId(1));
        assert_eq!(sm.current_scope().id, ScopeId(0)); // Still in top-level
    }

    #[test]
    fn test_scope_manager_enter_exit() {
        let mut sm = ScopeManager::new();

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });

        sm.enter_scope(class_id);
        assert_eq!(sm.current_scope().id, ScopeId(1));

        sm.exit_scope();
        assert_eq!(sm.current_scope().id, ScopeId(0));
    }

    #[test]
    fn test_scope_manager_local_var() {
        let mut sm = ScopeManager::new();

        sm.current_scope_mut().set_local_var("x".to_string(), VertexId(10));

        assert_eq!(sm.lookup_var("x"), Some(VertexId(10)));
        assert_eq!(sm.lookup_var("y"), None);
    }

    #[test]
    fn test_scope_manager_nested_lookup() {
        let mut sm = ScopeManager::new();

        // Top level: x = 10
        sm.current_scope_mut().set_local_var("x".to_string(), VertexId(10));

        // Enter class
        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        // Class level: y = 20
        sm.current_scope_mut().set_local_var("y".to_string(), VertexId(20));

        // Can lookup both x (from parent) and y (from current)
        assert_eq!(sm.lookup_var("x"), Some(VertexId(10)));
        assert_eq!(sm.lookup_var("y"), Some(VertexId(20)));
    }

    #[test]
    fn test_scope_manager_current_class_name() {
        let mut sm = ScopeManager::new();

        assert_eq!(sm.current_class_name(), None);

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        assert_eq!(sm.current_class_name(), Some("User".to_string()));

        // Enter method within class
        let method_id = sm.new_scope(ScopeKind::Method {
            name: "test".to_string(),
            receiver_type: None,
        });
        sm.enter_scope(method_id);

        // Should still find parent class name
        assert_eq!(sm.current_class_name(), Some("User".to_string()));
    }
}
