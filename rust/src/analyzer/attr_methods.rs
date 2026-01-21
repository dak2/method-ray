//! attr_reader, attr_writer, attr_accessor support
//!
//! Handles Ruby attribute methods that generate getter/setter methods.

use crate::env::GlobalEnv;
use crate::types::Type;
use ruby_prism::CallNode;

/// Process attr_reader call
/// attr_reader :name, :age generates:
/// - def name; @name; end
/// - def age; @age; end
pub fn process_attr_reader(genv: &mut GlobalEnv, call_node: &CallNode) {
    let class_name: String = match genv.scope_manager.current_class_name() {
        Some(name) => name,
        None => return, // attr_reader outside of class is ignored
    };

    let attr_names = extract_symbol_arguments(call_node);

    for attr_name in attr_names {
        let ivar_name = format!("@{}", attr_name);

        // Get instance variable type from class scope, or default to Bot (untyped)
        let return_type = genv
            .scope_manager
            .lookup_instance_var(&ivar_name)
            .and_then(|vtx| {
                genv.get_vertex(vtx).and_then(|v| {
                    // Get first type from vertex's types HashMap
                    v.types.keys().next().cloned()
                })
            })
            .unwrap_or(Type::Bot);

        // Register getter method: def name; @name; end
        let recv_ty = Type::Instance {
            class_name: class_name.clone(),
        };
        genv.register_builtin_method(recv_ty, &attr_name, return_type);
    }
}

/// Process attr_writer call
/// attr_writer :name generates:
/// - def name=(value); @name = value; end
pub fn process_attr_writer(genv: &mut GlobalEnv, call_node: &CallNode) {
    let class_name: String = match genv.scope_manager.current_class_name() {
        Some(name) => name,
        None => return,
    };

    let attr_names = extract_symbol_arguments(call_node);

    for attr_name in attr_names {
        let method_name = format!("{}=", attr_name);

        // Writer method returns the assigned value (Bot for now)
        let recv_ty = Type::Instance {
            class_name: class_name.clone(),
        };
        genv.register_builtin_method(recv_ty, &method_name, Type::Bot);
    }
}

/// Process attr_accessor call
/// attr_accessor :name is equivalent to attr_reader :name + attr_writer :name
pub fn process_attr_accessor(genv: &mut GlobalEnv, call_node: &CallNode) {
    process_attr_reader(genv, call_node);
    process_attr_writer(genv, call_node);
}

/// Extract symbol names from arguments
/// e.g., attr_reader :name, :age -> ["name", "age"]
fn extract_symbol_arguments(call_node: &CallNode) -> Vec<String> {
    let mut names = Vec::new();

    if let Some(arguments) = call_node.arguments() {
        for arg in &arguments.arguments() {
            if let Some(symbol_node) = arg.as_symbol_node() {
                let unescaped = symbol_node.unescaped();
                let name = String::from_utf8_lossy(&unescaped).to_string();
                names.push(name);
            }
        }
    }

    names
}

/// Check if a CallNode is an attr method and process it
/// Returns true if it was an attr method call
pub fn try_process_attr_method(genv: &mut GlobalEnv, call_node: &CallNode) -> bool {
    // Only handle receiver-less calls (attr_reader is called without explicit receiver)
    if call_node.receiver().is_some() {
        return false;
    }

    let method_name = String::from_utf8_lossy(call_node.name().as_slice()).to_string();

    match method_name.as_str() {
        "attr_reader" => {
            process_attr_reader(genv, call_node);
            true
        }
        "attr_writer" => {
            process_attr_writer(genv, call_node);
            true
        }
        "attr_accessor" => {
            process_attr_accessor(genv, call_node);
            true
        }
        _ => false,
    }
}
