//! Ruby FFI bindings for Method-Ray
//!
//! This module provides the Ruby gem interface using magnus.

use magnus::{function, prelude::*, Error, Ruby};
use methodray_core::{
    analyzer::AstInstaller,
    env::{GlobalEnv, LocalEnv},
    parser::ParseSession,
    rbs,
};

/// Type inference (public module function)
fn infer_types(source: String) -> Result<String, Error> {
    // Parse
    let session = ParseSession::new();
    let parse_result = session.parse_source(&source, "source.rb").map_err(|e| {
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

/// RBS cache generation (executed when requiring methodray/setup)
fn setup() -> Result<(), Error> {
    let ruby = unsafe { Ruby::get_unchecked() };
    let mut genv = GlobalEnv::new();

    // This will load RBS and save to cache if not already cached
    rbs::register_rbs_methods(&mut genv, &ruby)?;

    Ok(())
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("MethodRay")?;
    module.define_singleton_method("infer_types", function!(infer_types, 1))?;

    // require methodray/setup to generate RBS cache on Ruby side
    setup()?;

    Ok(())
}
