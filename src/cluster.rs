// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use serde::{Deserialize, Serialize};
use std::fs;
use crate::configuration::{ConfigError, HydrogenConfig};
use crate::node_id;

type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterNode {
    pub node_id: String,
    pub node_validation: u32,
    pub address: String,
    pub slots: [u32; 2],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub cluster_validation: u32,
    pub nodes: Vec<ClusterNode>,
}

pub fn generate_cluster_file(config: &HydrogenConfig) -> ConfigResult<()> {
    let cluster_node = ClusterNode {
        node_id: node_id::generate_node_id(),
        node_validation: 0,
        address: config.bind_address(),
        slots: [0, 16383],
    };

    let cluster_config = ClusterConfig {
        cluster_validation: 0,
        nodes: vec![cluster_node],
    };

    let content = serde_json::to_string_pretty(&cluster_config)?;
    fs::write("cluster.json", content)?;
    Ok(())
} 