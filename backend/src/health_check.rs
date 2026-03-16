use std::time::Duration;

use crate::{
    HEALTH_CHECK_INTERVAL_SECS,
    errors::ServerError,
    jsons,
    redis_handler
};


const HEALTH_CHECK_ENDPOINT: &str = "/payments/service-health";

pub async fn wait_for_services(redis_address: &str, df_address: &str, fb_address: &str) {
    let http_client = reqwest::Client::new();

    loop {
        let redis_ok = deadpool_redis::redis::Client::open(redis_address)
            .and_then(|c| c.get_connection())
            .map(|mut con| 
                deadpool_redis::redis::cmd("PING")
                .query::<String>(&mut con)
                .is_ok()
            )
            .unwrap_or(false);

        let df_ok = http_client.get(df_address).send().await.is_ok();
        let fb_ok = http_client.get(fb_address).send().await.is_ok();

        if redis_ok && df_ok && fb_ok {
            break;
        }

        print!(
            "Waiting for services... redis={} df={} fb={}",
            redis_ok, df_ok, fb_ok
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    //clean the line above
    print!("\x1B[2K\r");
}


/// Fetches the health check endpoint and returns whether the processor is alive.
/// Returns `false` on any error or if `failing` is true.
async fn fetch_is_alive(client: &reqwest::Client, address: &str) -> bool {
    let url = format!("{}{}", address, HEALTH_CHECK_ENDPOINT);
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(hc) = resp.json::<jsons::health_check::HealthCheck>().await {
                !hc.failing
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Periodically checks the health check endpoints of the default and fallback
/// processors, updating their respective health status in Redis.
///
/// Runs independently from the queue drainer.
pub async fn run_health_checker(
    pool: deadpool_redis::Pool,
    client: reqwest::Client,
    df_address: String,
    fb_address: String,
) -> Result<(), ServerError> {
    loop {
        tokio::time::sleep(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)).await;

        let mut con = match pool.get().await {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Health Checker: failed to get Redis connection: {}", err);
                continue;
            }
        };

        let df_alive = fetch_is_alive(&client, &df_address).await;
        let fb_alive = fetch_is_alive(&client, &fb_address).await;

        if let Err(err) = redis_handler::set_default_is_alive(&mut con, df_alive).await {
            eprintln!("Failed to set default health status: {}", err);
        }
        if let Err(err) = redis_handler::set_fallback_is_alive(&mut con, fb_alive).await {
            eprintln!("Failed to set fallback health status: {}", err);
        }
    }
}