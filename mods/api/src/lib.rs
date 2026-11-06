#[doc(hidden)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "squadron-mod",
        pub_export_macro: true,
        default_bindings_module: "squadron_mod_api::bindings",
    });
}

pub use bindings::squadron::api::host;
pub use bindings::squadron::api::types::{Output, Request, Response};
pub use bindings::{Agent, Guest};

pub trait Mod {
    fn on_event(_name: String, _agent: Option<Agent>, _data: String) {}

    fn run(action: String, _agent: Option<Agent>) -> Result<(), String> {
        Err(format!("This mod has no action {action}."))
    }

    fn call_tool(name: String, _input: String, _agent: Option<Agent>) -> Result<String, String> {
        Err(format!("This mod has no tool {name}."))
    }
}

#[macro_export]
macro_rules! squadron_mod {
    ($mod:ty) => {
        struct SquadronExport;

        impl $crate::Guest for SquadronExport {
            fn on_event(name: String, agent: Option<$crate::Agent>, data: String) {
                <$mod as $crate::Mod>::on_event(name, agent, data)
            }

            fn run(action: String, agent: Option<$crate::Agent>) -> Result<(), String> {
                <$mod as $crate::Mod>::run(action, agent)
            }

            fn call_tool(name: String, input: String, agent: Option<$crate::Agent>) -> Result<String, String> {
                <$mod as $crate::Mod>::call_tool(name, input, agent)
            }
        }

        $crate::bindings::export!(SquadronExport with_types_in $crate::bindings);
    };
}
