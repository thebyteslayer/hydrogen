// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use std::net::SocketAddr;
use tracing::info;

pub fn display_startup_info(server_addr: SocketAddr) {
    info!("Hydrogen running on {}", server_addr);
} 