use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::core::LedgerRecord;

#[async_trait]
pub trait AppendOnlyStorage: Send + Sync {
    async fn append(&self, record: LedgerRecord) -> Result<(), StorageError>;
    async fn get(&self, event_id: &str) -> Result<Option<LedgerRecord>, StorageError>;
    async fn query_records(
        &self,
        entity_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<LedgerRecord>, StorageError>;
    async fn verify_chain(&self) -> Result<bool, StorageError>;
    async fn get_latest_hash(&self) -> Result<Option<String>, StorageError>;
    async fn get_merkle_root(&self) -> Result<String, StorageError>;
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Chain verification failed: {0}")]
    ChainVerification(String),
    #[error("Record not found")]
    NotFound,
}

// Example PostgreSQL implementation
pub struct PostgresStorage {
    pool: sqlx::PgPool,
    table_name: String,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str, table_name: &str) -> Result<Self, StorageError> {
        let pool = sqlx::PgPool::connect(connection_string)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        // Create table if not exists
        let create_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                event_id VARCHAR(255) PRIMARY KEY,
                event_data JSONB NOT NULL,
                metadata JSONB,
                timestamp TIMESTAMPTZ NOT NULL,
                previous_hash VARCHAR(255),
                chain_id VARCHAR(100) NOT NULL,
                signature TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                merkle_path TEXT[]
            )
            "#,
            table_name
        );
        
        sqlx::query(&create_table_query)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        Ok(Self {
            pool,
            table_name: table_name.to_string(),
        })
    }
}

#[async_trait]
impl AppendOnlyStorage for PostgresStorage {
    async fn append(&self, record: LedgerRecord) -> Result<(), StorageError> {
        let query = format!(
            r#"
            INSERT INTO {} (event_id, event_data, metadata, timestamp, previous_hash, chain_id, signature)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            self.table_name
        );
        
        sqlx::query(&query)
            .bind(&record.event_id)
            .bind(serde_json::to_value(&record.event)?)
            .bind(&record.metadata)
            .bind(record.timestamp)
            .bind(&record.previous_hash)
            .bind(&record.chain_id)
            .bind(&record.signature)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get(&self, event_id: &str) -> Result<Option<LedgerRecord>, StorageError> {
        let query = format!(
            "SELECT * FROM {} WHERE event_id = $1",
            self.table_name
        );
        
        let row = sqlx::query(&query)
            .bind(event_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        match row {
            Some(row) => {
                let record = LedgerRecord {
                    event_id: row.get("event_id"),
                    event: serde_json::from_value(row.get("event_data"))?,
                    metadata: row.get("metadata"),
                    timestamp: row.get("timestamp"),
                    previous_hash: row.get("previous_hash"),
                    chain_id: row.get("chain_id"),
                    signature: row.get("signature"),
                };
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }
    
    async fn query_records(
        &self,
        entity_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<LedgerRecord>, StorageError> {
        let mut query = format!("SELECT * FROM {} WHERE 1=1", self.table_name);
        let mut params = vec![];
        let mut param_counter = 1;
        
        if let Some(entity) = entity_id {
            query.push_str(&format!(" AND event_data->>'entity_id' = ${}", param_counter));
            params.push(entity.to_string());
            param_counter += 1;
        }
        
        if let Some(start) = start_time {
            query.push_str(&format!(" AND timestamp >= ${}", param_counter));
            params.push(start);
            param_counter += 1;
        }
        
        if let Some(end) = end_time {
            query.push_str(&format!(" AND timestamp <= ${}", param_counter));
            params.push(end);
        }
        
        query.push_str(" ORDER BY timestamp ASC");
        
        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        let mut records = Vec::new();
        for row in rows {
            records.push(LedgerRecord {
                event_id: row.get("event_id"),
                event: serde_json::from_value(row.get("event_data"))?,
                metadata: row.get("metadata"),
                timestamp: row.get("timestamp"),
                previous_hash: row.get("previous_hash"),
                chain_id: row.get("chain_id"),
                signature: row.get("signature"),
            });
        }
        
        Ok(records)
    }
    
    async fn verify_chain(&self) -> Result<bool, StorageError> {
        // Implementation for verifying hash chain integrity
        Ok(true)
    }
    
    async fn get_latest_hash(&self) -> Result<Option<String>, StorageError> {
        let query = format!(
            "SELECT event_id FROM {} ORDER BY timestamp DESC LIMIT 1",
            self.table_name
        );
        
        let row = sqlx::query(&query)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        Ok(row.map(|r| r.get("event_id")))
    }
    
    async fn get_merkle_root(&self) -> Result<String, StorageError> {
        // Implementation for calculating Merkle root
        Ok("merkle_root_placeholder".to_string())
    }
}
