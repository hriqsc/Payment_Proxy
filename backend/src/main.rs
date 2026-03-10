#![allow(
    clippy::enum_variant_names,
    clippy::needless_return,
    clippy::unnecessary_operation
)]
use std::{collections::HashMap, sync::Arc};
use axum::{Json, Router, extract::{Query, State}, routing::{get, post}};
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
    let df_address = std::env::var("DF_ADDRESS").unwrap();
    let fb_address = std::env::var("FB_ADDRESS").unwrap();
    let redis_address = std::env::var("REDIS_ADDRESS").unwrap();

    let df_address_2 = df_address.clone();
    let fb_address_2 = fb_address.clone();
    let redis_address_2 = redis_address.clone();
    
    wait_for_services(
        redis_address.clone(),
        df_address.clone(),
        fb_address.clone(),
    ).await;

    println!("Services are up, starting proxy");

    let tunel = tokio::spawn(async move{
        if let Err(err) = run_tunel(
            df_address,
            fb_address,
            redis_address
        ).await {
            eprintln!("Tunel error: {}", err);
        }
    });

    let health = tokio::spawn(async move{
        if let Err(err) = run_health_checker(
            redis_address_2,
            df_address_2,
            fb_address_2
        ).await {
            eprintln!("Health checker error: {}", err);
        }
    });

    if let Err(err) = tokio::try_join!(tunel, health) {
        eprintln!("Task panicked: {}", err);
    }
}


async fn run_tunel(
    df_address: String,
    fb_address: String,
    redis_address: String
) -> Result<(), ServerError>{
    let lb = LoadBalancer::new().await;
    
    let app_state : Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::new(
        df_address,
         fb_address,
         Arc::new(Mutex::new(lb)),
         redis_address
    )?));

    let app = Router::new()
        .route("/payments", post(route_payment))
        .route("/payments-summary", get(payments_summary))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(std::env::var("BACKEND_ADDRESS")?).await?;
    axum::serve(listener, app).await?;
    Ok(())
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

    let mut server = match maybe_server {
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

    match server.process_request(state.clone(),payload.clone()).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(err) => {
            eprintln!("Failed to process payment: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}


pub async fn payments_summary(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<Mutex<AppState>>>,
) -> (StatusCode, Json<CheckRequest>) {

    let to : String = match params.get("to"){
        Some(f) => f.to_string(),
        None => return (StatusCode::BAD_REQUEST, Json(CheckRequest::default()))
    };

    let mut redis_con = match state.lock().await.redis_client.clone().get_connection(){
        Ok(con) => con,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default()))
    };

    let summary = match redis_handler::get_summary(&mut redis_con, params.get("from"), &to){
        Ok(summary) => summary,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(CheckRequest::default()))
    };

    (StatusCode::OK, Json(summary))
}

const HEALTH_CHECK_ENDPOINT : &str = "/payments/service-health";

async fn run_health_checker(
    redis_address: String,
    df_address: String,
    fb_address: String,
) -> Result<(), ServerError>{

    let hc_client = reqwest::Client::new();
    let redis_client = redis::Client::open(redis_address.clone())?;
    let mut con = redis_client.get_connection()?;
    loop{
        if redis::cmd("PING").query::<String>(&mut con).is_err() {
            con = match redis_client.get_connection() {
                Ok(c) => c,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            };
        }

        let df_resp = hc_client.get(df_address.clone() + HEALTH_CHECK_ENDPOINT).send().await?;
        let fb_resp = hc_client.get(fb_address.clone() + HEALTH_CHECK_ENDPOINT).send().await?;

        let df_hc : jsons::health_check::HealthCheck = df_resp.json().await?;
        let fb_hc : jsons::health_check::HealthCheck = fb_resp.json().await?;

        redis_handler::set_default_is_alive(&mut con, !df_hc.failing)?;
        redis_handler::set_fallback_is_alive(&mut con, !fb_hc.failing)?;

        let not_processed = redis_handler::get_not_processed(&mut con)?;

        for payment in not_processed{
            match server::handle_request(
                payment.clone(),
                &df_address,
                &fb_address,
                &hc_client,
                &mut con, 
            ).await{
                Ok(_) => {},
                Err(_) => {
                    redis_handler::add_not_processed(
                        &mut con, &payment
                    )?;
                }
            };
            //sleep so it doesnt make too many requests in a short time
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

}



async fn wait_for_services(redis_address: String, df_address: String, fb_address: String) {
    let http_client = reqwest::Client::new();

    loop {
        let redis_ok = redis::Client::open(redis_address.as_str())
            .and_then(|c| c.get_connection())
            .map(|mut con| redis::cmd("PING").query::<String>(&mut con).is_ok())
            .unwrap_or(false);

        let df_ok = http_client.get(df_address.as_str()).send().await.is_ok();
        let fb_ok = http_client.get(fb_address.as_str()).send().await.is_ok();

        if redis_ok && df_ok && fb_ok {
            println!("All services are up, starting server...");
            break;
        }

        print!("Waiting for services... redis={} df={} fb={}\r", redis_ok, df_ok, fb_ok);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    //limpa linha acima
    print!("\x1B[2K\r");
}