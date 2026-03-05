#![allow(
    clippy::enum_variant_names,
    clippy::needless_return,
    clippy::unnecessary_operation
)]
use std::sync::Arc;
use axum::{routing::post, routing::get, Router, Json, extract::State};
use jsons::{check_request::CheckRequest, payment_request::PaymentRequest};
use reqwest::StatusCode;
use tokio::sync::Mutex;
use crate::{errors::ServerError, load_balancer::LoadBalancer};
use app_state::AppState;

mod load_balancer;
mod server;
mod errors;
mod jsons;
mod app_state;
mod redis_handler;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    
    match run_server().await{
        Ok(_) => {},
        Err(err) => {
            println!("{}", err)
        }
    }
}


async fn run_server() -> Result<(), ServerError>{
    let lb = LoadBalancer::new().await;
    
    let df_address = String::from("http://localhost:5000");
    let fb_address = String::from("http://localhost:5001");
    let redis_address = String::from("redis://127.0.0.1/");
    let app_state : Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::new(
        df_address,
         fb_address,
         Arc::new(Mutex::new(lb)),
         redis_address
    )?));

    let app = Router::new()
        .route("/pagamentos", post(route_payment))
        .route("/count", get(get_count))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9999").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn get_count(
    State(state): State<Arc<Mutex<AppState>>>,
) -> (StatusCode, Json<CheckRequest>) {
    let redis_cli = state.lock().await.redis_client.clone();

    let df_payments: isize = match redis_handler::get_df_payments(redis_cli.clone()).await {
        Ok(payments) => payments.unwrap_or_default(),
        Err(err) => {
            eprintln!("Failed to get default payments: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default()));
        }
    };

    let fb_payments: isize = match redis_handler::get_fb_payments(redis_cli.clone()).await {
        Ok(payments) => payments.unwrap_or_default(),
        Err(err) => {
            eprintln!("Failed to get fallback payments: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default()));
        }
    };

    let check = CheckRequest {
        default: df_payments,
        fallback: fb_payments,
    };

    (StatusCode::OK, Json(check))
}

pub async fn route_payment(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<PaymentRequest>,
) -> StatusCode {
    let lb = {
        let app = state.lock().await;
        app.load_balancer.clone()
    };

    let maybe_server = lb.lock().await.select_server().await;

    let server = match maybe_server {
        Ok(Some(sv)) => sv,
        Ok(None) => {
            eprintln!("No healthy servers available");
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        Err(err) => {
            eprintln!("Load balancer error: {}", err);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    server.add_p_request(payload.clone()).await;

    match server.process_p_request(state.clone()).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(err) => {
            eprintln!("Failed to process payment: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
