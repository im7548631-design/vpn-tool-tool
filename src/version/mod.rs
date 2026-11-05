use cli_engine::{CommandResult, CommandSpec, RuntimeCommandSpec};
use crate::next_action::next_action;

pub fn add_command() -> CommandSpec {
    CommandSpec::Runtime(RuntimeCommandSpec {
        name: "version".to_string(),
        description: "Show version number".to_string(),
        handler: Box::new(|_| {
            println!("1.0.0");
            
            next_action();
            
            Ok(CommandResult::Success)
        }),
    })
}
