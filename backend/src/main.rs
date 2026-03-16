#![allow(
    clippy::enum_variant_names,
    clippy::needless_return,
    clippy::unnecessary_operation
)]
use std::{collections::HashMap, sync::Arc, time::Duration};
use axum::{Json, Router, extract::{Query, State}, routing::{get, post}};
use jsons::{check_request::CheckRequest, payment_request::PaymentRequest};
use reqwest::StatusCode;
use tokio::sync::mpsc;
use crate::processor::PaymentJob;
use app_state::AppState;

mod errors;
mod jsons;
mod app_state;
mod redis_handler;
mod processor;
mod health_check;

const WORKER_COUNT: usize = 10;
const CHANNEL_BUFFER: usize = 10_000;
const HEALTH_CHECK_INTERVAL_SECS: u64 = 5;
const REQUEST_TIMEOUT_SECS: u64 = 3;

#[tokio::main]
async fn main() {
    let df_address = std::env::var("DF_ADDRESS").expect("DF_ADDRESS not set");
    let fb_address = std::env::var("FB_ADDRESS").expect("FB_ADDRESS not set");
    let redis_address = std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS not set");
    let backend_address = std::env::var("BACKEND_ADDRESS").expect("BACKEND_ADDRESS not set");

    health_check::wait_for_services(&redis_address, &df_address, &fb_address).await;
    println!("All services ready. Starting payment proxy...");

    let redis_pool = redis_handler::build_redis_pool(&redis_address).expect("Failed to build Redis pool");

    let (payment_tx, payment_rx) = mpsc::channel::<PaymentJob>(CHANNEL_BUFFER);

    let state = Arc::new(
        AppState::new(
            df_address.clone(),
            fb_address.clone(),
            redis_pool.clone(),
            payment_tx,
        )
        .expect("Failed to build AppState"),
    );

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .expect("Failed to build HTTP client");

    let payment_rx = Arc::new(tokio::sync::Mutex::new(payment_rx));
    for _ in 0..WORKER_COUNT {
        let rx = payment_rx.clone();
        let tx = state.payment_tx.clone();
        let pool = redis_pool.clone();
        let client = http_client.clone();
        let df = df_address.clone();
        let fb = fb_address.clone();
        tokio::spawn(async move {
            processor::worker_loop(rx, tx, pool, client, df, fb).await;
        });
    }

    // Health checker — only responsible for updating processor status in Redis
    {
        let pool = redis_pool.clone();
        let client = http_client.clone();
        let df = df_address.clone();
        let fb = fb_address.clone();
        tokio::spawn(async move {
            if let Err(err) = health_check::run_health_checker(pool, client, df, fb).await {
                eprintln!("Health checker error: {}", err);
            }
        });
    }

    // Queue drainer — independent task that retries payments saved in Redis
    {
        let pool = redis_pool.clone();
        let tx = state.payment_tx.clone();
        tokio::spawn(async move {
            processor::run_queue_drainer(pool, tx).await;
        });
    }

    // Run HTTP server
    let app = Router::new()
        .route("/payments", post(route_payment))
        .route("/payments-summary", get(payments_summary))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&backend_address)
        .await
        .expect("Failed to bind listener");

    println!("Listening on {}", backend_address);
    axum::serve(listener, app).await.expect("Server error");
}


// ---------------------------------------------------------------------------
// HTTP Handlers
// ---------------------------------------------------------------------------

/// Accepts a payment request and dispatches it to the worker channel.
/// Returns 202 Accepted immediately — processing is fire-and-forget.
pub async fn route_payment(
    State(state): State<Arc<AppState>>,
    Json(payment): Json<PaymentRequest>,
) -> StatusCode {
    match state.payment_tx.try_send(PaymentJob::new(payment)) {
        Ok(_) => StatusCode::ACCEPTED,
        Err(mpsc::error::TrySendError::Full(_)) => {
            eprintln!("Payment channel is full, dropping request");
            StatusCode::SERVICE_UNAVAILABLE
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            eprintln!("Payment channel is closed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Returns a summary of processed payments within the given time range.
pub async fn payments_summary(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CheckRequest>) {
    let to = match params.get("to") {
        Some(t) => t.clone(),
        None => return (StatusCode::BAD_REQUEST, Json(CheckRequest::default())),
    };

    let mut con = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default())),
    };

    match redis_handler::get_summary(&mut con, params.get("from"), &to).await {
        Ok(summary) => (StatusCode::OK, Json(summary)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default())),
    }
}



