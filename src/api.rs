// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use crate::cache::{CacheError, Hydrogen};
use crate::api_log::{log_set_endpoint, log_get_endpoint, log_delete_endpoint, log_keys_endpoint, log_invalid_endpoint};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Invalid command format: {0}")]
    InvalidCommand(String),
    #[error("Cache error: {0}")]
    CacheError(#[from] CacheError),
    #[error("Network error: {0}")]
    NetworkError(#[from] std::io::Error),
    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Delete { key: String },
    Keys,
}

impl Command {
    pub fn parse(input: &str) -> ApiResult<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ApiError::InvalidCommand("Empty command".to_string()));
        }

        let (command, rest) = match input.find(' ') {
            Some(pos) => (&input[..pos], input[pos+1..].trim()),
            None => (input, ""),
        };

        match command.to_uppercase().as_str() {
            "SET" => {
                let (key, value) = Self::parse_set_args(rest)?;
                Self::validate_key(&key)?;
                Ok(Command::Set { key, value })
            }
            "GET" => {
                if rest.is_empty() {
                    return Err(ApiError::InvalidCommand(
                        "GET command requires exactly one key".to_string(),
                    ));
                }
                let key = rest.to_string();
                Self::validate_key(&key)?;
                Ok(Command::Get { key })
            }
            "DEL" | "DELETE" => {
                if rest.is_empty() {
                    return Err(ApiError::InvalidCommand(
                        "DEL command requires exactly one key".to_string(),
                    ));
                }
                let key = rest.to_string();
                Self::validate_key(&key)?;
                Ok(Command::Delete { key })
            }
            "KEYS" => {
                if !rest.is_empty() {
                    return Err(ApiError::InvalidCommand(
                        "KEYS command takes no arguments".to_string(),
                    ));
                }
                Ok(Command::Keys)
            }
            cmd => Err(ApiError::InvalidCommand(format!(
                "Unknown command: {}. Supported commands: SET, GET, DEL, KEYS",
                cmd
            ))),
        }
    }

    fn parse_set_args(args: &str) -> ApiResult<(String, String)> {
        if args.is_empty() {
            return Err(ApiError::InvalidCommand(
                "SET command requires key and value".to_string(),
            ));
        }

        let (key, rest) = match args.find(' ') {
            Some(pos) => (&args[..pos], args[pos+1..].trim()),
            None => return Err(ApiError::InvalidCommand(
                "SET command requires key and value".to_string(),
            )),
        };

        if rest.is_empty() {
            return Err(ApiError::InvalidCommand(
                "SET command requires key and value".to_string(),
            ));
        }

        let value = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
            rest[1..rest.len()-1].to_string()
        } else {
            rest.split_whitespace().collect::<Vec<&str>>().join(" ")
        };

        Ok((key.to_string(), value))
    }

    fn validate_key(key: &str) -> ApiResult<()> {
        if key.is_empty() {
            return Err(ApiError::InvalidCommand("Key cannot be empty".to_string()));
        }

        // Check for spaces
        if key.contains(' ') {
            return Err(ApiError::InvalidCommand("Key cannot contain spaces".to_string()));
        }

        // Check each character
        for ch in key.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return Err(ApiError::InvalidCommand(format!(
                    "Key contains invalid character '{}'. Keys can only contain letters, numbers, hyphens, and underscores",
                    ch
                )));
            }
        }

        // Check that - and _ are not at the beginning, end, or alone
        let chars: Vec<char> = key.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if ch == '-' || ch == '_' {
                // Cannot be at the beginning or end
                if i == 0 || i == chars.len() - 1 {
                    return Err(ApiError::InvalidCommand(format!(
                        "Key cannot start or end with '{}'. Hyphens and underscores must be between letters or numbers",
                        ch
                    )));
                }
                
                // Must be between alphanumeric characters
                let prev = chars[i - 1];
                let next = chars[i + 1];
                if !prev.is_ascii_alphanumeric() || !next.is_ascii_alphanumeric() {
                    return Err(ApiError::InvalidCommand(format!(
                        "Invalid key format. '{}' must be between letters or numbers",
                        ch
                    )));
                }
            }
        }

        // Check for consecutive - or _
        for i in 0..chars.len() - 1 {
            if (chars[i] == '-' || chars[i] == '_') && (chars[i + 1] == '-' || chars[i + 1] == '_') {
                return Err(ApiError::InvalidCommand(
                    "Key cannot have consecutive hyphens or underscores".to_string()
                ));
            }
        }

        Ok(())
    }
}

pub struct TcpApiServer {
    cache: Arc<Hydrogen>,
    listener: TcpListener,
}

impl TcpApiServer {
    pub async fn new(bind_addr: &str, cache: Arc<Hydrogen>) -> ApiResult<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        Ok(Self { cache, listener })
    }

    pub async fn run(&self) -> ApiResult<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, client_addr)) => {
                    let cache = Arc::clone(&self.cache);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, cache, client_addr).await {
                            error!("Error handling client {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting TCP connection: {}", e);
                }
            }
        }
    }

    async fn handle_client(stream: TcpStream, cache: Arc<Hydrogen>, client_addr: SocketAddr) -> ApiResult<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let request_str = line.trim();
                    if request_str.is_empty() {
                        continue;
                    }
                    
                    let response = match Command::parse(request_str) {
                        Ok(command) => {
                            match &command {
                                Command::Set { key, value } => {
                                    log_set_endpoint(key, value);
                                }
                                Command::Get { key } => {
                                    log_get_endpoint(key);
                                }
                                Command::Delete { key } => {
                                    log_delete_endpoint(key);
                                }
                                Command::Keys => {
                                    log_keys_endpoint();
                                }
                            }
                            Self::execute_command(command, &cache).await
                        }
                        Err(_) => {
                            log_invalid_endpoint(request_str);
                            format!("ERROR: Invalid endpoint format")
                        }
                    };
                    
                    let response_with_newline = format!("{}\n", response);
                    if let Err(e) = writer.write_all(response_with_newline.as_bytes()).await {
                        error!("Failed to send response to {}: {}", client_addr, e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading from TCP stream {}: {}", client_addr, e);
                    break;
                }
            }
        }
        
        Ok(())
    }



    async fn execute_command(command: Command, cache: &Hydrogen) -> String {
        match command {
            Command::Set { key, value } => {
                match cache.set(key.clone(), value).await {
                    Ok(()) => "OK".to_string(),
                    Err(e) => format!("ERROR: {}", e)
                }
            }
            Command::Get { key } => {
                match cache.get(&key).await {
                    Ok(value) => value,
                    Err(CacheError::KeyNotFound(_)) => "NULL".to_string(),
                    Err(e) => format!("ERROR: {}", e)
                }
            }
            Command::Delete { key } => {
                match cache.delete(&key).await {
                    Ok(existed) => {
                        if existed {
                            "1".to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    Err(e) => format!("ERROR: {}", e)
                }
            }
            Command::Keys => {
                match cache.keys().await {
                    Ok(keys) => {
                        if keys.is_empty() {
                            "(empty)".to_string()
                        } else {
                            keys.join(" ")
                        }
                    }
                    Err(e) => format!("ERROR: {}", e)
                }
            }
        }
    }

    pub fn local_addr(&self) -> ApiResult<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
}

