use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use crate::{
    HEALTH_CHECK_INTERVAL_SECS, errors::ServerError, jsons::payment_request::{PaymentRequest, ProcessorPRequest}, redis_handler
};

const PAYMENT_ENDPOINT: &str = "/payments";
const DEFAULT_MAX_RETRIES: u8 = 3;

pub struct PaymentJob {
    pub payment: PaymentRequest,
    pub default_attempts: u8,
}

impl PaymentJob {
    pub fn new(payment: PaymentRequest) -> PaymentJob {
        PaymentJob { payment, default_attempts: 0 }
    }

    pub fn add_attempt(payment: PaymentRequest, attempts: u8) -> PaymentJob {
        PaymentJob { payment, default_attempts: attempts }
    }
}

/// Worker loop — consumes jobs from the channel.
///
/// On a default processor failure, requeues the job with `default_attempts + 1`
/// instead of sleeping and retrying synchronously. This keeps the worker free
/// to process other payments immediately.
///
/// Once `DEFAULT_MAX_RETRIES` is exhausted, tries the fallback once.
/// If the fallback also fails, saves to Redis for later retry.
pub async fn worker_loop(
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<PaymentJob>>>,
    tx: mpsc::Sender<PaymentJob>,
    pool: deadpool_redis::Pool,
    client: reqwest::Client,
    df_address: String,
    fb_address: String,
) {
    loop {
        let job = {
            let mut guard = rx.lock().await;
            guard.recv().await
        };

        let job = match job {
            Some(j) => j,
            None => break,
        };

        let mut con = match pool.get().await {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Worker failed to get Redis connection: {}", err);
                let _ = tx.try_send(job);
                continue;
            }
        };

        let df_alive = redis_handler::get_default_is_alive(&mut con).await.unwrap_or(false);

        //it tries to send the payment to the default processor
        if df_alive && job.default_attempts < DEFAULT_MAX_RETRIES {
            match send_payment(
                &client, 
                &df_address, 
                &ProcessorPRequest::new(&job.payment)
            ).await {
                Ok(true) => {
                    let _ = redis_handler::add_payment(&mut con, true, job.payment.amount).await;
                    continue;
                }
                _ => {
                    let _ = tx.try_send(PaymentJob::add_attempt(job.payment, job.default_attempts + 1));
                    continue;
                }
            }
        }

        // Default exhausted or dead — try fallback once
        let fb_alive = redis_handler::get_fallback_is_alive(&mut con).await.unwrap_or(false);
        if fb_alive {
            match send_payment(
                &client, 
                &fb_address, 
                &ProcessorPRequest::new(&job.payment)
            ).await {
                Ok(true) => {
                    let _ = redis_handler::add_payment(&mut con, false, job.payment.amount).await;
                    continue;
                }
                _ => {}
            }
        }

        // Both failed — persist for retry by the queue drainer
        eprintln!("Both processors unavailable, queuing payment for retry");
        if let Err(err) = redis_handler::add_not_processed(&mut con, &job.payment).await {
            eprintln!("Failed to persist unprocessed payment: {}", err);
        }
    }
}

/// Sends a single payment request to the given processor URL.
/// Returns `Ok(true)` on HTTP 2xx, `Ok(false)` on other status codes.
async fn send_payment(
    client: &reqwest::Client,
    base_addr: &str,
    request: &ProcessorPRequest,
) -> Result<bool, ServerError> {
    let response = client
        .post(format!("{}{}", base_addr, PAYMENT_ENDPOINT))
        .json(request)
        .send()
        .await?;

    Ok(response.status().is_success())
}


/// Periodically drains the Redis not-processed queue and requeues payments
/// back into the worker channel. Runs independently from the health checker.
pub async fn run_queue_drainer(
    pool: deadpool_redis::Pool,
    tx: mpsc::Sender<PaymentJob>,
) {
    loop {
        tokio::time::sleep(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)).await;

        let mut con = match pool.get().await {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Queue drainer failed to get Redis connection: {}", err);
                continue;
            }
        };

        let pending = match redis_handler::pop_not_processed(&mut con).await {
            Ok(p) => p,
            Err(err) => {
                eprintln!("Queue drainer failed to fetch pending payments: {}", err);
                continue;
            }
        };

        for payment in pending {
            if tx.try_send(PaymentJob::new(payment)).is_err() {
                eprintln!("Queue drainer: channel full, payment dropped from retry");
            }
        }
    }
}