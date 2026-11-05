use cli_engine::{CommandResult, CommandSpec, RuntimeCommandSpec};
use crate::next_action::next_action;
use crate::auth::require_login;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, LINK};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

fn get_all_flag_values(args: &[String], flag: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            if let Some(val) = iter.next() {
                values.push(val.clone());
            }
        } else if arg.starts_with(&format!("{}=", flag)) {
            values.push(arg[flag.len() + 1..].to_string());
        }
    }
    values
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag || arg.starts_with(&format!("{}=", flag)))
}

fn extract_next_link(link_header: &str) -> Option<String> {
    for part in link_header.split(',') {
        let sections: Vec<&str> = part.split(';').collect();
        if sections.len() >= 2 {
            let url_part = sections[0].trim();
            let rel_part = sections[1].trim();
            if rel_part.contains("rel=\"next\"") || rel_part.contains("rel=next") {
                if url_part.starts_with('<') && url_part.ends_with('>') {
                    return Some(url_part[1..url_part.len() - 1].to_string());
                }
            }
        }
    }
    None
}

fn resolve_url(input_url: &str, hostname_opt: Option<&str>) -> String {
    if input_url.starts_with("http://") || input_url.starts_with("https://") {
        return input_url.to_string();
    }

    let hostname = hostname_opt
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GITLAB_HOSTNAME").ok())
        .unwrap_or_else(|| "gitlab.com".to_string());

    let base = if hostname.starts_with("http://") || hostname.starts_with("https://") {
        hostname
    } else {
        format!("https://{}", hostname)
    };

    if input_url.starts_with('/') {
        format!("{}{}", base, input_url)
    } else {
        format!("{}/{}", base.trim_end_matches('/'), input_url)
    }
}

pub fn add_command() -> CommandSpec {
    CommandSpec::Runtime(RuntimeCommandSpec {
        name: "call".to_string(),
        description: "Make an authenticated API request".to_string(),
        handler: Box::new(|args| {
            require_login();

            // Extract flags
            let url_opt = get_flag_value(args, "--url");
            let hostname_opt = get_flag_value(args, "--hostname");
            let method_opt = get_flag_value(args, "-X").or_else(|| get_flag_value(args, "--method"));
            let header_vals = get_all_flag_values(args, "-H").into_iter().chain(get_all_flag_values(args, "--header"));
            let field_vals = get_all_flag_values(args, "-F").into_iter().chain(get_all_flag_values(args, "--field"));
            let raw_field_vals = get_all_flag_values(args, "-f").into_iter().chain(get_all_flag_values(args, "--raw-field"));
            let input_file = get_flag_value(args, "--input");
            let output_format = get_flag_value(args, "--output").unwrap_or_else(|| "json".to_string());
            let include_headers = has_flag(args, "-i") || has_flag(args, "--include");
            let silent = has_flag(args, "--silent");
            let paginate = has_flag(args, "--paginate");
            let dry_run = has_flag(args, "--dry-run");

            let raw_url = match url_opt {
                Some(u) => u,
                None => {
                    eprintln!("error: required option '--url <url>' not specified");
                    std::process::exit(1);
                }
            };

            let initial_url = resolve_url(&raw_url, hostname_opt.as_deref());

            // Determine default method based on flags (-F or --field changes default to POST)
            let has_fields = !field_vals.clone().collect::<Vec<_>>().is_empty() 
                || !raw_field_vals.clone().collect::<Vec<_>>().is_empty();
            
            let method = method_opt.unwrap_or_else(|| {
                if has_fields { "POST".to_string() } else { "GET".to_string() }
            });

            // Handle Dry Run
            if dry_run {
                println!("=== [DRY RUN] ===");
                println!("Method:   {}", method.to_uppercase());
                println!("Raw URL:  {}", raw_url);
                println!("Full URL: {}", initial_url);
                if let Some(ref h) = hostname_opt {
                    println!("Hostname: {}", h);
                }
                
                let headers: Vec<_> = header_vals.collect();
                if !headers.is_empty() {
                    println!("Headers:");
                    for h in headers {
                        println!("  {}", h);
                    }
                }

                let fields: Vec<_> = field_vals.chain(raw_field_vals).collect();
                if !fields.is_empty() {
                    println!("Fields/Body Parameters:");
                    for f in fields {
                        println!("  {}", f);
                    }
                }

                if let Some(ref input) = input_file {
                    println!("Input File: {}", input);
                }

                println!("Paginate: {}", paginate);
                println!("Output Format: {}", output_format);
                println!("==================");

                next_action();
                return Ok(CommandResult::Success);
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                let client = reqwest::Client::new();
                let mut current_url = Some(initial_url);
                let mut page_count = 0;

                while let Some(url) = current_url.take() {
                    page_count += 1;
                    let mut req_builder = match method.to_uppercase().as_str() {
                        "POST" => client.post(&url),
                        "PUT" => client.put(&url),
                        "DELETE" => client.delete(&url),
                        "PATCH" => client.patch(&url),
                        _ => client.get(&url),
                    };

                    // Add headers
                    let mut headers = HeaderMap::new();
                    for h in header_vals.clone() {
                        if let Some((k, v)) = h.split_once(':') {
                            if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(k.trim().as_bytes()), HeaderValue::from_str(v.trim())) {
                                headers.insert(name, val);
                            }
                        }
                    }
                    req_builder = req_builder.headers(headers);

                    // Add input body if specified
                    if page_count == 1 {
                        if let Some(ref filepath) = input_file {
                            if filepath == "-" {
                                use std::io::{self, Read};
                                let mut buffer = String::new();
                                if io::stdin().read_to_string(&mut buffer).is_ok() {
                                    req_builder = req_builder.body(buffer);
                                }
                            } else if Path::new(&filepath).exists() {
                                if let Ok(content) = fs::read_to_string(filepath) {
                                    req_builder = req_builder.body(content);
                                }
                            }
                        }

                        // Add fields as JSON body if present and no input file
                        let mut json_fields = HashMap::new();
                        for f in field_vals.clone() {
                            if let Some((k, v)) = f.split_once('=') {
                                json_fields.insert(k.to_string(), v.to_string());
                            }
                        }
                        for f in raw_field_vals.clone() {
                            if let Some((k, v)) = f.split_once('=') {
                                json_fields.insert(k.to_string(), v.to_string());
                            }
                        }
                        if !json_fields.is_empty() && input_file.is_none() {
                            req_builder = req_builder.json(&json_fields);
                        }
                    }

                    match req_builder.send().await {
                        Ok(resp) => {
                            let status = resp.status();
                            let resp_headers = resp.headers().clone();

                            if include_headers {
                                println!("HTTP/1.1 {}", status);
                                for (k, v) in resp_headers.iter() {
                                    println!("{}: {}", k, v.to_str().unwrap_or(""));
                                }
                                println!();
                            }

                            let next_url = if paginate {
                                if let Some(link_val) = resp_headers.get(LINK) {
                                    if let Ok(link_str) = link_val.to_str() {
                                        extract_next_link(link_str)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if !silent {
                                if let Ok(text) = resp.text().await {
                                    println!("{}", text);
                                }
                            }

                            if paginate {
                                current_url = next_url;
                            }
                        }
                        Err(e) => {
                            eprintln!("API request failed on page {}: {}", page_count, e);
                            std::process::exit(1);
                        }
                    }
                }

                Ok::<(), reqwest::Error>(())
            });

            if let Err(e) = result {
                eprintln!("Pagination workflow failed: {}", e);
                std::process::exit(1);
            }

            next_action();
            Ok(CommandResult::Success)
        }),
    })
}
