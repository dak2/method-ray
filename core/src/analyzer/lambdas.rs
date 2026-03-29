//! Lambdas/Procs - type inference for lambda and proc literals
//!
//! Handles `-> { }`, `lambda { }`, `proc { }`, and `Proc.new { }`.

use ruby_prism::{LambdaNode, Node};

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

pub(crate) fn process_lambda_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    lambda_node: &LambdaNode,
) -> Option<VertexId> {
    process_proc_body(genv, lenv, changes, source, lambda_node.parameters(), lambda_node.body())
}

pub(crate) fn process_block_as_proc(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    block_node: &ruby_prism::BlockNode,
) -> Option<VertexId> {
    process_proc_body(genv, lenv, changes, source, block_node.parameters(), block_node.body())
}

fn process_proc_body<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    parameters: Option<Node<'a>>,
    body: Option<Node<'a>>,
) -> Option<VertexId> {
    let (_scope_id, merge_vtx) = genv.enter_lambda();

    let param_vtxs = parameters
        .and_then(|p| p.as_block_parameters_node())
        .map(|bp| {
            super::blocks::install_block_parameters_with_vtxs(genv, lenv, changes, source, &bp)
        })
        .unwrap_or_default();

    let body_vtx = body.and_then(|b| {
        if let Some(stmts) = b.as_statements_node() {
            super::install::install_statements(genv, lenv, changes, source, &stmts)
        } else {
            super::install::install_node(genv, lenv, changes, source, &b)
        }
    });

    if let Some(vtx) = body_vtx {
        genv.add_edge(vtx, merge_vtx);
    }

    genv.exit_scope();

    let proc_ty = Type::proc_type_with_vertex(merge_vtx, param_vtxs);
    let proc_vtx = genv.new_source(proc_ty);

    Some(proc_vtx)
}

#[cfg(test)]
mod tests {
    use crate::analyzer::AstInstaller;
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::parser::ParseSession;
    use crate::types::Type;

    fn parse_and_install(source: &str) -> GlobalEnv {
        parse_and_install_with(source, |_| {})
    }

    fn parse_and_install_with_builtin(source: &str) -> GlobalEnv {
        parse_and_install_with(source, |genv| {
            genv.register_builtin_method(Type::string(), "upcase", Type::string());
        })
    }

    fn parse_and_install_with(source: &str, setup: impl FnOnce(&mut GlobalEnv)) -> GlobalEnv {
        let session = ParseSession::new();
        let result = session.parse_source(source, "<test>").unwrap();
        let mut genv = GlobalEnv::new();
        setup(&mut genv);
        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = result.node();
        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }
        installer.finish();
        genv
    }

    #[test]
    fn test_lambda_basic_no_crash() {
        let genv = parse_and_install("f = -> { 42 }");
        assert!(genv.type_errors.is_empty());
    }

    #[test]
    fn test_lambda_with_params_no_crash() {
        let genv = parse_and_install("f = -> (x) { x }");
        assert!(genv.type_errors.is_empty());
    }

    #[test]
    fn test_lambda_body_type_error_detected() {
        let genv = parse_and_install_with_builtin("f = -> { 42.upcase }");
        assert!(
            !genv.type_errors.is_empty(),
            "Expected type error for 42.upcase inside lambda"
        );
    }

    #[test]
    fn test_lambda_method_basic() {
        let genv = parse_and_install("f = lambda { 42 }");
        assert!(genv.type_errors.is_empty());
    }

    #[test]
    fn test_proc_method_basic() {
        let genv = parse_and_install("f = proc { 42 }");
        assert!(genv.type_errors.is_empty());
    }

    #[test]
    fn test_proc_new_basic() {
        let genv = parse_and_install("f = Proc.new { 42 }");
        assert!(genv.type_errors.is_empty());
    }

    #[test]
    fn test_lambda_call_arg_type_propagation() {
        let genv = parse_and_install_with_builtin(
            "f = ->(x) { x.upcase }\nf.call(42)",
        );
        assert!(
            !genv.type_errors.is_empty(),
            "Expected type error for 42.upcase via lambda arg propagation"
        );
    }
}
