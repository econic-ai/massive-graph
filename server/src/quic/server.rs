//! QUIC server implementation

use std::sync::Arc;
use massive_graph_core::core::factory::ConfiguredAppState;
use massive_graph_core::{log_info};
use massive_graph_core::core::config::QuicConfig;
use massive_graph_core::types::storage::{ChunkStorage, DeltaStreamChunk};

use crate::quic::connection_manager::{ConnectionManager, create_quic_server};
use crate::quic::shard_runtime::ShardRuntime;
use crate::quic::types::ShardId;

/// QUIC ingress service
pub struct QuicService {
    config: QuicConfig,
    storage: Arc<ChunkStorage<DeltaStreamChunk>>,
}

impl QuicService {
    /// Create new QUIC service
    pub fn new(config: QuicConfig) -> Self {
        // TODO: For POC, create a simple ChunkStorage instance
        // In real implementation, would get from Store<S>
        let chunk_storage = Arc::new(ChunkStorage::new());
        
        Self {
            config,
            storage: chunk_storage,
        }
    }
    
    /// Run the QUIC service
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        log_info!("Starting QUIC service on {}", self.config.bind_address);
        
        // Create QUIC server
        let server = create_quic_server(&self.config).await?;
        
        // Create shard runtimes
        let mut shards = Vec::with_capacity(self.config.shard_count as usize);
        for shard_idx in 0..self.config.shard_count {
            let shard_id = ShardId(shard_idx);
            let shard = Arc::new(ShardRuntime::new(
                shard_id,
                self.storage.clone(),
                self.config.workers_per_shard,
            ));
            shards.push(shard);
        }
        
        log_info!(
            "Created {} shards with {} workers each",
            self.config.shard_count,
            self.config.workers_per_shard
        );
        
        // Create and run connection manager
        let conn_mgr = ConnectionManager::new(
            server,
            shards,
            self.config.shard_count,
        );
        
        conn_mgr.run().await?;
        
        Ok(())
    }
}

/// Entry point for running QUIC service from main.rs
pub async fn run_quic_service(
    configured_app_state: ConfiguredAppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = configured_app_state.quic_config();
    if !config.enabled {
        log_info!("QUIC service disabled in configuration");
        return Ok(());
    }
    
    let service = QuicService::new(config);
    service.run().await
}

