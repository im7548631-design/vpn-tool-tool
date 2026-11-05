use cli_engine::{CommandResult, CommandSpec, CommandSpecGroupSpec, RuntimeCommandSpec};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use sha2::{Digest, Sha256};
use crate::next_action::next_action;

fn config_dir() -> PathBuf {
    let mut dir = dirs::home_dir().expect("Could not find home directory");
    dir.push(".vpn-tool");
    dir
}

fn auth_file() -> PathBuf {
    let mut path = config_dir();
    path.push("auth.json");
    path
}

fn session_file() -> PathBuf {
    let mut path = config_dir();
    path.push("session");
    path
}

fn ensure_config() -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&dir)?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&dir, perms)?;
    }
    
    Ok(())
}

fn hash_pin(pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn registered() -> bool {
    auth_file().exists()
}

fn logged_in() -> bool {
    if !session_file().exists() {
        return false;
    }
    if let Ok(content) = fs::read_to_string(session_file()) {
        return content.trim() == "logged_in";
    }
    false
}

fn load_auth() -> Option<serde_json::Value> {
    if !registered() {
        return None;
    }
    let content = fs::read_to_string(auth_file()).ok()?;
    serde_json::from_str(&content).ok()
}

fn ask(question: &str) -> io::Result<String> {
    print!("{}", question);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[allow(dead_code)]
pub fn require_login() {
    if !registered() {
        eprintln!("Not registered. Run: vpn-tool auth register");
        std::process::exit(1);
    }

    if !logged_in() {
        eprintln!("Not logged in. Run: vpn-tool auth login");
        std::process::exit(1);
    }
}

pub fn add_command() -> CommandSpec {
    CommandSpec::Group(CommandSpecGroupSpec {
        name: "auth".to_string(),
        description: "Authentication commands".to_string(),
        subcommands: vec![
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "register".to_string(),
                description: "Register a new PIN".to_string(),
                handler: Box::new(|_| {
                    if registered() {
                        eprintln!("PIN already registered.");
                        std::process::exit(1);
                    }

                    let pin = ask("Enter new PIN: ").unwrap_or_default();
                    if pin.is_empty() {
                        eprintln!("PIN cannot be empty.");
                        std::process::exit(1);
                    }

                    let confirm = ask("Confirm new PIN: ").unwrap_or_default();
                    if pin != confirm {
                        eprintln!("PINs do not match.");
                        std::process::exit(1);
                    }

                    if let Err(e) = ensure_config() {
                        eprintln!("Failed to create config directory: {}", e);
                        std::process::exit(1);
                    }

                    let data = serde_json::json!({
                        "version": 1,
                        "pin_hash": hash_pin(&pin)
                    });

                    if let Err(e) = fs::write(auth_file(), serde_json::to_string_pretty(&data).unwrap()) {
                        eprintln!("Failed to write auth file: {}", e);
                        std::process::exit(1);
                    }

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(auth_file(), fs::Permissions::from_mode(0o600));
                    }

                    let _ = fs::write(session_file(), "logged_out");

                    println!("PIN registered successfully.");
                    println!("Run: vpn-tool auth login");

                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "login".to_string(),
                description: "Login with PIN".to_string(),
                handler: Box::new(|_| {
                    if !registered() {
                        eprintln!("No PIN registered. Run: vpn-tool auth register");
                        std::process::exit(1);
                    }

                    let auth_data = load_auth();
                    let pin = ask("Enter PIN: ").unwrap_or_default();

                    let valid = auth_data
                        .as_ref()
                        .and_then(|d| d.get("pin_hash"))
                        .and_then(|h| h.as_str())
                        .map_or(false, |h| h == hash_pin(&pin));

                    if !valid {
                        eprintln!("Invalid PIN.");
                        std::process::exit(1);
                    }

                    let _ = ensure_config();
                    let _ = fs::write(session_file(), "logged_in");
                    println!("Login successful.");

                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "logout".to_string(),
                description: "Logout".to_string(),
                handler: Box::new(|_| {
                    if !registered() {
                        eprintln!("No PIN registered.");
                        std::process::exit(1);
                    }

                    let auth_data = load_auth();
                    let pin = ask("Enter PIN: ").unwrap_or_default();

                    let valid = auth_data
                        .as_ref()
                        .and_then(|d| d.get("pin_hash"))
                        .and_then(|h| h.as_str())
                        .map_or(false, |h| h == hash_pin(&pin));

                    if !valid {
                        eprintln!("Invalid PIN.");
                        std::process::exit(1);
                    }

                    let _ = fs::write(session_file(), "logged_out");
                    println!("Logout successful.");

                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "status".to_string(),
                description: "Show authentication status".to_string(),
                handler: Box::new(|_| {
                    println!("Registered: {}", if registered() { "yes" } else { "no" });
                    println!("Logged in:  {}", if logged_in() { "yes" } else { "no" });
                    
                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "reset".to_string(),
                description: "Reset PIN".to_string(),
                handler: Box::new(|_| {
                    if !registered() {
                        eprintln!("No PIN registered.");
                        std::process::exit(1);
                    }

                    let auth_data = load_auth();
                    let current = ask("Enter current PIN: ").unwrap_or_default();

                    let valid = auth_data
                        .as_ref()
                        .and_then(|d| d.get("pin_hash"))
                        .and_then(|h| h.as_str())
                        .map_or(false, |h| h == hash_pin(&current));

                    if !valid {
                        eprintln!("Invalid current PIN.");
                        std::process::exit(1);
                    }

                    let new_pin = ask("Enter new PIN: ").unwrap_or_default();
                    if new_pin.is_empty() {
                        eprintln!("New PIN cannot be empty.");
                        std::process::exit(1);
                    }

                    let confirm = ask("Confirm new PIN: ").unwrap_or_default();
                    if new_pin != confirm {
                        eprintln!("PINs do not match.");
                        std::process::exit(1);
                    }

                    let data = serde_json::json!({
                        "version": 1,
                        "pin_hash": hash_pin(&new_pin)
                    });

                    let _ = fs::write(auth_file(), serde_json::to_string_pretty(&data).unwrap());
                    let _ = fs::write(session_file(), "logged_out");
                    println!("PIN reset successfully.");

                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
            CommandSpec::Runtime(RuntimeCommandSpec {
                name: "remove".to_string(),
                description: "Remove authentication".to_string(),
                handler: Box::new(|_| {
                    if !registered() {
                        eprintln!("No PIN registered.");
                        std::process::exit(1);
                    }

                    let auth_data = load_auth();
                    let pin = ask("Enter PIN: ").unwrap_or_default();

                    let valid = auth_data
                        .as_ref()
                        .and_then(|d| d.get("pin_hash"))
                        .and_then(|h| h.as_str())
                        .map_or(false, |h| h == hash_pin(&pin));

                    if !valid {
                        eprintln!("Invalid PIN.");
                        std::process::exit(1);
                    }

                    let _ = fs::remove_file(auth_file());
                    let _ = fs::remove_file(session_file());
                    println!("Authentication removed.");

                    next_action();

                    Ok(CommandResult::Success)
                }),
            }),
        ],
    })
}
