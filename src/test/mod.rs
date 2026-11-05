use cli_engine::{CommandResult, CommandSpec, RuntimeCommandSpec};
use crate::next_action::next_action;
use crate::auth::require_login;

const API_BASE: &str = "https://vpn-gateway-manager--jjajajjajahsh.replit.app";

async fn api_request(method: &str, endpoint: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", API_BASE, endpoint);
    match method {
        "POST" => client.post(&url).send().await,
        _ => client.get(&url).send().await,
    }
}

fn get_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        } else if arg.starts_with(&format!("{}=", flag)) {
            return Some(arg[flag.len() + 1..].to_string());
        }
    }
    None
}

pub fn add_command() -> CommandSpec {
    CommandSpec::Runtime(RuntimeCommandSpec {
        name: "test".to_string(),
        description: "Test API delay".to_string(),
        handler: Box::new(|args| {
            require_login();
            let ms = get_flag_value(args, "--ms").unwrap_or_else(|| "100".to_string());

            let rt = tokio::runtime::Runtime::new().unwrap();
            let endpoint = format!("/api/vpn/test/sleep?ms={}", urlencoding::encode(&ms));
            let result = rt.block_on(async { api_request("GET", &endpoint).await });

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
