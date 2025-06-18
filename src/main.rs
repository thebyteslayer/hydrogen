// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

mod api;
mod cache;
mod environment;
mod logging;

use api::TcpApiServer;
use cache::Hydrogen;
use environment::HydrogenConfig;
use logging::display_startup_info;
use std::sync::Arc;
use tracing::error;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    let config = HydrogenConfig::load_or_create()?;
    let bind_addr = config.bind_address();
    
    let cache = Arc::new(Hydrogen::new());
    let server = TcpApiServer::new(&bind_addr, cache.clone()).await?;
    
    display_startup_info(server.local_addr()?);
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
        }
    }

    Ok(())
}
