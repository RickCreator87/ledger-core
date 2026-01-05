use axum::{
    routing::{get, post},
    Router, Json, extract::State,
};
use crate::core::{DigitalLedger, LedgerError};
use crate::core::event::LedgerEvent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    pub ledger: Arc<DigitalLedger>,
}

pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/events", post(append_event))
        .route("/audit", get(get_audit_trail))
        .route("/integrity", get(verify_integrity))
        .route("/merkle-root", get(get_merkle_root))
        .with_state(state)
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

#[derive(Deserialize)]
struct AppendEventRequest {
    event: LedgerEvent,
    metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct AppendEventResponse {
    event_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    status: String,
}

async fn append_event(
    State(state): State<ApiState>,
    Json(payload): Json<AppendEventRequest>,
) -> Result<Json<AppendEventResponse>, LedgerError> {
    let event_id = state.ledger.append_event(payload.event, payload.metadata).await?;
    
    Ok(Json(AppendEventResponse {
        event_id,
        timestamp: chrono::Utc::now(),
        status: "appended".to_string(),
    }))
}

async fn get_audit_trail(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::core::LedgerRecord>>, LedgerError> {
    let records = state.ledger.get_audit_trail(None, None, None).await?;
    Ok(Json(records))
}

async fn verify_integrity(
    State(state): State<ApiState>,
) -> Result<Json<IntegrityResponse>, LedgerError> {
    let is_valid = state.ledger.verify_integrity().await?;
    
    Ok(Json(IntegrityResponse {
        is_valid,
        verified_at: chrono::Utc::now(),
        message: if is_valid {
            "Ledger integrity verified".to_string()
        } else {
            "Ledger integrity check failed".to_string()
        },
    }))
}

async fn get_merkle_root(
    State(state): State<ApiState>,
) -> Result<Json<MerkleRootResponse>, LedgerError> {
    let root = state.ledger.get_merkle_root().await?;
    
    Ok(Json(MerkleRootResponse {
        merkle_root: root,
        timestamp: chrono::Utc::now(),
    }))
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct IntegrityResponse {
    is_valid: bool,
    verified_at: chrono::DateTime<chrono::Utc>,
    message: String,
}

#[derive(Serialize)]
struct MerkleRootResponse {
    merkle_root: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}
