use cli_engine::{CommandSpec, CommandSpecGroupSpec};

// Import submodules within the api module
pub mod call;

pub fn add_command() -> CommandSpec {
    CommandSpec::Group(CommandSpecGroupSpec {
        name: "api".to_string(),
        description: "Perform authenticated API requests and operations".to_string(),
        subcommands: vec![
            call::add_command(),
        ],
    })
}
