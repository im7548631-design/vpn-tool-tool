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
        name: "connect".to_string(),
        description: "Connect VPN".to_string(),
        handler: Box::new(|args| {
            require_login();
            let country = get_flag_value(args, "--country").unwrap_or_else(|| {
                eprintln!("error: required option '--country <code>' not specified");
                std::process::exit(1);
            });

            let rt = tokio::runtime::Runtime::new().unwrap();
            let endpoint = format!("/api/vpn/connect?countryCode={}", urlencoding::encode(&country));
            let result = rt.block_on(async { api_request("POST", &endpoint).await });

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
