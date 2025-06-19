use std::io::{self, Write, BufRead, BufReader};
use std::net::TcpStream;

#[derive(Debug)]
enum ParsedCommand {
    Quit,
    Help,
    Set { address: String, key: String, value: String },
    Get { address: String, key: String },
    Del { address: String, key: String },
    Keys { address: String },
    Error(String),
}

fn parse_command_line(input: &str) -> Option<ParsedCommand> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "quit" | "exit" => Some(ParsedCommand::Quit),
        "help" => Some(ParsedCommand::Help),
        _ => {
            // First part should be address
            let address = parts[0].to_string();
            if parts.len() < 2 {
                return None;
            }

            let command = parts[1];
            match command {
                "set" => {
                    if parts.len() < 4 {
                        return Some(ParsedCommand::Error(format!("Usage: {} set <key> <value>", address)));
                    }

                    let key = parts[2];

                    // Handle quoted values
                    let value = if input.contains('"') {
                        parse_quoted_value(input)?
                    } else {
                        parts[3..].join(" ")
                    };

                    Some(ParsedCommand::Set { address, key: key.to_string(), value })
                }
                "get" => {
                    if parts.len() != 3 {
                        return Some(ParsedCommand::Error(format!("Usage: {} get <key>", address)));
                    }
                    let key = parts[2];
                    Some(ParsedCommand::Get { address, key: key.to_string() })
                }
                "del" => {
                    if parts.len() != 3 {
                        return Some(ParsedCommand::Error(format!("Usage: {} del <key>", address)));
                    }
                    let key = parts[2];
                    Some(ParsedCommand::Del { address, key: key.to_string() })
                }
                "keys" => {
                    if parts.len() != 2 {
                        return Some(ParsedCommand::Error(format!("Usage: {} keys", address)));
                    }
                    Some(ParsedCommand::Keys { address })
                }
                _ => Some(ParsedCommand::Error(format!("Unknown command: {}", command))),
            }
        }
    }
}

fn parse_quoted_value(input: &str) -> Option<String> {
    if let Some(start) = input.find('"') {
        if let Some(end) = input.rfind('"') {
            if start != end {
                return Some(input[start+1..end].to_string());
            }
        }
    }
    None
}



fn main() {
    let stdin = io::stdin();
    loop {
        print!("hydrogen-cli> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                
                match parse_command_line(input) {
                    Some(ParsedCommand::Quit) => {
                        println!("Goodbye!");
                        break;
                    }
                    Some(ParsedCommand::Help) => {
                        println!("Available commands:");
                        println!("  <ip:port> set <key> <value>      - Set a key-value pair");
                        println!("  <ip:port> set <key> \"<value>\"    - Set a key-value pair with spaces");
                        println!("  <ip:port> get <key>              - Get value for a key");
                        println!("  <ip:port> del <key>              - Delete a key");
                        println!("  <ip:port> keys                   - List all keys in the cache");
                        println!("  help                             - Show this help message");
                        println!("  quit/exit                        - Exit the CLI");
                    }
                    Some(ParsedCommand::Set { address, key, value }) => {
                        execute_command(&address, &format!("SET {} \"{}\"", key, value));
                    }
                    Some(ParsedCommand::Get { address, key }) => {
                        execute_command(&address, &format!("GET {}", key));
                    }
                    Some(ParsedCommand::Del { address, key }) => {
                        execute_command(&address, &format!("DEL {}", key));
                    }
                    Some(ParsedCommand::Keys { address }) => {
                        execute_command(&address, "KEYS");
                    }
                    Some(ParsedCommand::Error(msg)) => {
                        println!("{}", msg);
                    }
                    None => {
                        println!("Usage: <ip:port> <command> [args...]");
                        println!("Type 'help' for available commands.");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

fn execute_command(address: &str, command: &str) {
    match TcpStream::connect(address) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(command.as_bytes()) {
                println!("Failed to send command: {}", e);
                return;
            }

            if let Err(e) = stream.write_all(b"\n") {
                println!("Failed to send newline: {}", e);
                return;
            }

            let mut reader = BufReader::new(&mut stream);
            let mut response = String::new();
            match reader.read_line(&mut response) {
                Ok(_) => {
                    let trimmed = response.trim();
                    if !trimmed.is_empty() {
                        println!("{}", trimmed);
                    }
                }
                Err(e) => {
                    println!("Failed to read response: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to connect to {}: {}", address, e);
        }
    }
} 