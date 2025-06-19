// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use rand::Rng;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const NODE_ID_LENGTH: usize = 7;

pub fn generate_node_id() -> String {
    let mut rng = rand::thread_rng();
    (0..NODE_ID_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
} 