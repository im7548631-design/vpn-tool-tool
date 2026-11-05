use cli_engine::{CommandResult, CommandSpec, RuntimeCommandSpec};
use crate::next_action::next_action;
// Assuming require_login is accessible from crate::auth or defined/imported accordingly
use crate::auth::require_login;

const API_BASE: &str = "https://vpn-gateway-manager--jjajajjajahsh.replit.app";

async fn api_request(method: &str, endpoint: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", API_BASE, endpoint);
    
    match method {
        "GET" => client.get(&url).send().await,
        "POST" => client.post(&url).send().await,
        _ => client.get(&url).send().await,
    }
}

pub fn add_command() -> CommandSpec {
    CommandSpec::Runtime(RuntimeCommandSpec {
        name: "healthz".to_string(),
        description: "Check API health".to_string(),
        handler: Box::new(|_| {
            // Require login before performing the action
            require_login();

            // Run async API request within a block or block_on if synchronous handler is expected
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                api_request("GET", "/api/healthz").await
            });

            match result {
                Ok(resp) => {
                    println!("Status: {}", resp.status());
                    if let Ok(text) = resp.text().await {
                        println!("{}", text);
                    }
                }
                Err(e) => {
                    eprintln!("API request failed: {}", e);
                    std::process::exit(1);
                }
            }

            next_action();

            Ok(CommandResult::Success)
        }),
    })
}
