mod engine;
mod queue;
mod executor;
mod oracle;
mod position_manager;
mod insurance;
mod models;
mod errors;

use engine::LiquidationEngine;
use tracing_subscriber::EnvFilter;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::queue::LiquidationQueue;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct PendingLiquidationsResponse {
    positions: Vec<String>, // IDs of at-risk positions
}

#[derive(Clone)]
struct AppState {
    queue: Arc<LiquidationQueue>,
}

#[tokio::main]
async fn main() {
    // logging setup
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    println!("Starting liquidation backend...");

    // create engine
    let engine = LiquidationEngine::new();
    let queue = engine.queue.clone();

    // spawn engine loop in background
    tokio::spawn(async move {
        engine.start().await;
    });

    // build HTTP API router with shared state
    let state = AppState { queue };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/liquidations/pending", get(pending_liquidations_handler))
        .with_state(state);

    // bind server
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    println!("HTTP server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn pending_liquidations_handler(
    State(state): State<AppState>,
) -> Json<PendingLiquidationsResponse> {
    let ids = state.queue.snapshot_ids();

    Json(PendingLiquidationsResponse {
        positions: ids,
    })
}

