use std::sync::Arc;
use tokio::sync::mpsc;
use crate::{errors::ServerError, processor::PaymentJob};

/// Shared application state used across request handlers.
pub struct AppState {
    /// Address of the default payment processor.
    pub default_address: Arc<str>,

    /// Address of the fallback payment processor.
    pub fallback_address: Arc<str>,

    /// Redis connection pool.
    pub redis_pool: deadpool_redis::Pool,

    /// Sender side of the payment processing channel.
    pub payment_tx: mpsc::Sender<PaymentJob>,
}

impl AppState {
    pub fn new(
        default_address: String,
        fallback_address: String,
        redis_pool: deadpool_redis::Pool,
        payment_tx: mpsc::Sender<PaymentJob>,
    ) -> Result<AppState, ServerError> {
        Ok(AppState {
            default_address: Arc::from(default_address),
            fallback_address: Arc::from(fallback_address),
            redis_pool,
            payment_tx,
        })
    }
}