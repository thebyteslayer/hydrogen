// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use zstd::{decode_all, encode_all};

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Compression failed: {0}")]
    CompressionError(String),
    #[error("Decompression failed: {0}")]
    DecompressionError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

type CacheResult<T> = Result<T, CacheError>;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub compressed_data: Vec<u8>,
}

impl CacheEntry {
    pub fn new(value: &str) -> CacheResult<Self> {
        let compressed_data = encode_all(value.as_bytes(), 3)
            .map_err(|e| CacheError::CompressionError(e.to_string()))?;
        
        Ok(Self {
            compressed_data,
        })
    }

    pub fn get_value(&self) -> CacheResult<String> {
        let decompressed = decode_all(&self.compressed_data[..])
            .map_err(|e| CacheError::DecompressionError(e.to_string()))?;
        
        String::from_utf8(decompressed)
            .map_err(|e| CacheError::DecompressionError(format!("UTF-8 error: {}", e)))
    }
}

#[derive(Debug)]
pub struct Hydrogen {
    storage: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl Hydrogen {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set(&self, key: String, value: String) -> CacheResult<()> {
        let entry = CacheEntry::new(&value)?;
        let mut storage = self.storage.write().await;
        storage.insert(key.clone(), entry);
        Ok(())
    }

    pub async fn get(&self, key: &str) -> CacheResult<String> {
        let storage = self.storage.read().await;
        match storage.get(key) {
            Some(entry) => {
                let value = entry.get_value()?;
                Ok(value)
            }
            None => {
                Err(CacheError::KeyNotFound(key.to_string()))
            }
        }
    }

    pub async fn delete(&self, key: &str) -> CacheResult<bool> {
        let mut storage = self.storage.write().await;
        let existed = storage.remove(key).is_some();
        Ok(existed)
    }

    pub async fn keys(&self) -> CacheResult<Vec<String>> {
        let storage = self.storage.read().await;
        let keys: Vec<String> = storage.keys().cloned().collect();
        Ok(keys)
    }




}

impl Default for Hydrogen {
    fn default() -> Self {
        Self::new()
    }
}

