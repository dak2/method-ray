use magnus::{function, method, prelude::*, Error, Ruby};

mod types;
mod parser;

// Phase 1: 型推論・メソッド探索機能実装中

#[magnus::wrap(class = "MethodRay::Analyzer")]
struct Analyzer {
    path: String,
}

impl Analyzer {
    fn new(path: String) -> Self {
        Self { path }
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    // MethodRay モジュールを定義
    let module = ruby.define_module("MethodRay")?;

    // MethodRay::Analyzer クラスを定義
    let class = module.define_class("Analyzer", ruby.class_object())?;

    // new メソッド（コンストラクタ）
    class.define_singleton_method("new", function!(Analyzer::new, 1))?;

    // version メソッド（Phase 0のテスト用）
    class.define_method("version", method!(Analyzer::version, 0))?;

    Ok(())
}
