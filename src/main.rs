use gitdigital_ledger_core::{
    api,
    core::DigitalLedger,
    storage::{append_only::PostgresStorage, AppendOnlyStorage},
    compliance::validator::{ComplianceValidator, AmountLimitRule, SanctionedCountriesRule},
};
use std::sync::Arc;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load configuration
    let config = load_config().await?;
    
    // Initialize storage
    let storage = PostgresStorage::new(&config.database_url, "ledger_events")
        .await
        .map_err(|e| format!("Failed to initialize storage: {}", e))?;
    
    // Initialize compliance validator
    let mut validator = ComplianceValidator::new();
    
    // Add compliance rules
    validator.add_rule(Box::new(AmountLimitRule::new(
        rust_decimal::Decimal::new(1000000, 0), // 1,000,000
        "USD",
    )));
    
    validator.add_rule(Box::new(SanctionedCountriesRule::new(vec![
        "CU", "IR", "KP", "SY",
    ])));
    
    // Create ledger
    let ledger = DigitalLedger::new(
        Arc::new(storage),
        Arc::new(validator),
        "main_ledger".to_string(),
    )
    .await
    .map_err(|e| format!("Failed to create ledger: {}", e))?;
    
    // Create API state
    let state = api::routes::ApiState {
        ledger: Arc::new(ledger),
    };
    
    // Start API server
    let app = api::routes::create_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    
    tracing::info!("Server listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Load configuration from environment or file
    Ok(Config {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://user:pass@localhost/ledger".to_string()),
    })
}

struct Config {
    database_url: String,
}
