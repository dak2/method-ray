// Core modules - always available
mod types;
mod parser;
mod graph;
mod env;
mod analyzer;
mod rbs;
mod cache;
pub mod diagnostics;
mod source_map;

// Feature-gated modules
#[cfg(any(feature = "cli", feature = "lsp"))]
pub mod checker;

#[cfg(feature = "lsp")]
pub mod lsp;

// Ruby FFI - only when building as cdylib (not for CLI or LSP)
#[cfg(not(any(feature = "cli", feature = "lsp")))]
use magnus::{function, method, prelude::*, Error, Ruby};

#[cfg(not(any(feature = "cli", feature = "lsp")))]
use env::GlobalEnv;

// Ruby gem implementation - only when not building CLI or LSP
#[cfg(not(any(feature = "cli", feature = "lsp")))]
mod ruby_impl {
    use super::*;
    use magnus::{function, method, prelude::*, Error, Ruby};
    use env::GlobalEnv;

    #[magnus::wrap(class = "MethodRay::Analyzer")]
    pub struct Analyzer {
        path: String,
    }

    impl Analyzer {
        fn new(path: String) -> Self {
            Self { path }
        }

        fn version(&self) -> String {
            "0.1.0".to_string()
        }

        /// Execute type inference
        fn infer_types(&self, source: String) -> Result<String, Error> {
            use analyzer::AstInstaller;
            use env::LocalEnv;

            // Parse
            let parse_result = parser::parse_ruby_source(&source, "source.rb".to_string())
                .map_err(|e| {
                    let ruby = unsafe { Ruby::get_unchecked() };
                    Error::new(ruby.exception_runtime_error(), e.to_string())
                })?;

            // Build graph
            let mut genv = GlobalEnv::new();

            // Register built-in methods from RBS
            let ruby = unsafe { Ruby::get_unchecked() };
            rbs::register_rbs_methods(&mut genv, &ruby)?;

            let mut lenv = LocalEnv::new();
            let mut installer = AstInstaller::new(&mut genv, &mut lenv, &source);

            // Process AST
            let root = parse_result.node();
            if let Some(program_node) = root.as_program_node() {
                let statements = program_node.statements();
                for stmt in &statements.body() {
                    installer.install_node(&stmt);
                }
            }

            installer.finish();

            // Return results as string
            let mut results = Vec::new();
            for (var_name, vtx_id) in lenv.all_vars() {
                if let Some(vtx) = genv.get_vertex(*vtx_id) {
                    results.push(format!("{}: {}", var_name, vtx.show()));
                }
            }

            Ok(results.join("\n"))
        }
    }

    #[magnus::init]
    pub fn init(ruby: &Ruby) -> Result<(), Error> {
        let module = ruby.define_module("MethodRay")?;
        let class = module.define_class("Analyzer", ruby.class_object())?;

        class.define_singleton_method("new", function!(Analyzer::new, 1))?;
        class.define_method("version", method!(Analyzer::version, 0))?;
        class.define_method("infer_types", method!(Analyzer::infer_types, 1))?;

        Ok(())
    }
}
