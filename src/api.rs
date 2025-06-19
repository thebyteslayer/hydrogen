// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use crate::cache::{CacheError, Hydrogen};
use crate::api_log::{log_set_endpoint, log_get_endpoint, log_delete_endpoint, log_invalid_endpoint, log_invalid_utf8};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
}

impl Command {
    pub fn parse(input: &str) -> ApiResult<Self> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(ApiError::InvalidCommand("Empty command".to_string()));
        }

        match parts[0].to_uppercase().as_str() {
            "SET" => {
                if parts.len() < 3 {
                    return Err(ApiError::InvalidCommand(
                        "SET command requires key and value".to_string(),
                    ));
                }
                let key = parts[1].to_string();
                let value = parts[2..].join(" ");
                Ok(Command::Set { key, value })
            }
            "GET" => {
                if parts.len() != 2 {
                    return Err(ApiError::InvalidCommand(
                        "GET command requires exactly one key".to_string(),
                    ));
                }
                let key = parts[1].to_string();
                Ok(Command::Get { key })
            }
            "DEL" | "DELETE" => {
                if parts.len() != 2 {
                    return Err(ApiError::InvalidCommand(
                        "DEL command requires exactly one key".to_string(),
                    ));
                }
                let key = parts[1].to_string();
                Ok(Command::Delete { key })
            }
            cmd => Err(ApiError::InvalidCommand(format!(
                "Unknown command: {}. Supported commands: SET, GET, DEL",
                cmd
            ))),
        }
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

    async fn handle_client(mut stream: TcpStream, cache: Arc<Hydrogen>, client_addr: SocketAddr) -> ApiResult<()> {
        let mut buffer = vec![0u8; 8192];
        
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(len) => {
                    let request = buffer[..len].to_vec();
                    
                    let response = match String::from_utf8(request) {
                        Ok(request_str) => {
                            let request_str = request_str.trim();
                            if request_str.is_empty() {
                                continue;
                            }
                            
                            match Command::parse(request_str) {
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
                                    }
                                    Self::execute_command(command, &cache).await
                                }
                                Err(_) => {
                                    log_invalid_endpoint(request_str);
                                    format!("ERROR: Invalid endpoint format")
                                }
                            }
                        }
                        Err(_) => {
                            log_invalid_utf8();
                            format!("ERROR: Invalid UTF-8")
                        }
                    };
                    
                    let response_with_newline = format!("{}\n", response);
                    if let Err(e) = stream.write_all(response_with_newline.as_bytes()).await {
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
        }
    }

    pub fn local_addr(&self) -> ApiResult<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
}

