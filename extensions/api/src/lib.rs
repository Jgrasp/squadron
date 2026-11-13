#[doc(hidden)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "squadron-extension",
        pub_export_macro: true,
        default_bindings_module: "squadron_extension_api::bindings",
    });
}

pub use bindings::squadron::api::host;
pub use bindings::squadron::api::types::{Account, AgentState, BytesResponse, Output, Request, Response, SavedFile};
pub use bindings::{Agent, Guest};

pub trait Extension {
    fn on_event(_name: String, _agent: Option<Agent>, _data: String) {}

    fn run(action: String, _agent: Option<Agent>) -> Result<(), String> {
        Err(format!("This extension has no action {action}."))
    }

    fn call_tool(name: String, _input: String, _agent: Option<Agent>) -> Result<String, String> {
        Err(format!("This extension has no tool {name}."))
    }
}

#[macro_export]
macro_rules! squadron_extension {
    ($extension:ty) => {
        struct SquadronExport;

        impl $crate::Guest for SquadronExport {
            fn on_event(name: String, agent: Option<$crate::Agent>, data: String) {
                <$extension as $crate::Extension>::on_event(name, agent, data)
            }

            fn run(action: String, agent: Option<$crate::Agent>) -> Result<(), String> {
                <$extension as $crate::Extension>::run(action, agent)
            }

            fn call_tool(name: String, input: String, agent: Option<$crate::Agent>) -> Result<String, String> {
                <$extension as $crate::Extension>::call_tool(name, input, agent)
            }
        }

        $crate::bindings::export!(SquadronExport with_types_in $crate::bindings);
    };
}
