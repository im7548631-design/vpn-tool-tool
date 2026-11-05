use cli_engine::{BuildInfo, Cli, CliConfig, Module};
use crate::next_action::next_action;

mod auth;
mod api;
mod version;
mod next_action;
mod countries;
mod create;
mod connect;
mod disconnect;
mod status;
mod test;
mod ip;
mod healthz;

fn main() {
    let build_info = BuildInfo {
        name: "vpn-tool".to_string(),
        version: "1.0.0".to_string(),
        about: "Node.js-based VPN CLI tool ported to Rust".to_string(),
    };

    let mut cli = Cli::new(CliConfig {
        build_info,
    });

    // Register modules and individual commands directly at the root level (no vpn group)
    cli.add_command(auth::add_command());
    cli.add_command(api::add_command());
    cli.add_command(version::add_command());
    cli.add_command(countries::add_command());
    cli.add_command(create::add_command());
    cli.add_command(connect::add_command());
    cli.add_command(disconnect::add_command());
    cli.add_command(status::add_command());
    cli.add_command(test::add_command());
    cli.add_command(ip::add_command());
    cli.add_command(healthz::add_command());

    // Run the CLI engine with built-in --version support
    cli.run();
}
