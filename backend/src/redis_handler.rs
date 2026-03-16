use deadpool_redis::redis::AsyncCommands;
use crate::{
    errors::ServerError,
    jsons::{
        check_request::{CheckRequest, ProcessorSummary},
        payment_request::PaymentRequest,
    },
};
use chrono::DateTime;

pub const KEY_DEFAULT: &str = "payments:default";
pub const KEY_FALLBACK: &str = "payments:fallback";
pub const DEFAULT_IS_ALIVE: &str = "default_is_alive";
pub const FALLBACK_IS_ALIVE: &str = "fallback_is_alive";
pub const NOT_PROCESSED: &str = "not_processed_payments";

#[derive(serde::Serialize, serde::Deserialize)]
struct SummaryEntry {
    amount: f64,
    timestamp: u64,
}

// ---------------------------------------------------------------------------
// Not-processed queue
// ---------------------------------------------------------------------------

pub fn build_redis_pool(redis_address: &str) -> Result<deadpool_redis::Pool, ServerError> {
    let cfg = deadpool_redis::Config::from_url(redis_address);
    cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .map_err(|e| ServerError::RedisClientError(e.to_string()))
}

/// Adds a payment request to the not-processed queue for retry.
pub async fn add_not_processed(
    con: &mut deadpool_redis::Connection,
    payment: &PaymentRequest,
) -> Result<(), ServerError> {
    let value = serde_json::to_string(payment)?;
    con.lpush::<&str,String,usize>(NOT_PROCESSED, value).await?;
    Ok(())
}

/// Atomically pops all pending payments from the retry queue.
pub async fn pop_not_processed(
    con: &mut deadpool_redis::Connection,
) -> Result<Vec<PaymentRequest>, ServerError> {
    let temp_key = format!("{}:processing", NOT_PROCESSED);

    let renamed: bool = deadpool_redis::redis::cmd("RENAME")
        .arg(NOT_PROCESSED)
        .arg(&temp_key)
        .query_async(con.as_mut())
        .await
        .unwrap_or(false);

    if !renamed {
        return Ok(vec![]);
    }

    let entries: Vec<String> = con.lrange(&temp_key, 0, -1).await?;
    con.del::<&str,usize>(&temp_key).await?;

    let payments = entries
        .iter()
        .filter_map(|e| serde_json::from_str(e).ok())
        .collect();

    Ok(payments)
}

// ---------------------------------------------------------------------------
// Processor health
// ---------------------------------------------------------------------------

pub async fn get_default_is_alive(con: &mut deadpool_redis::Connection) -> Result<bool, ServerError> {
    let val: Option<String> = con.get(DEFAULT_IS_ALIVE).await?;
    match val {
        Some(s) => Ok(s.parse()?),
        None => Ok(false),
    }
}

pub async fn get_fallback_is_alive(con: &mut deadpool_redis::Connection) -> Result<bool, ServerError> {
    let val: Option<String> = con.get(FALLBACK_IS_ALIVE).await?;
    match val {
        Some(s) => Ok(s.parse()?),
        None => Ok(false),
    }
}

pub async fn set_default_is_alive(
    con: &mut deadpool_redis::Connection,
    is_alive: bool,
) -> Result<(), ServerError> {
    con.set::<&str,String,String>(
        DEFAULT_IS_ALIVE, 
        is_alive.to_string()
    ).await?;
    Ok(())
}

pub async fn set_fallback_is_alive(
    con: &mut deadpool_redis::Connection,
    is_alive: bool,
) -> Result<(), ServerError> {
    con.set::<&str,String,String>(
        FALLBACK_IS_ALIVE, 
        is_alive.to_string()
    ).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Payment recording
// ---------------------------------------------------------------------------

/// Records a payment in the default or fallback processor's summary.
pub async fn add_payment(
    con: &mut deadpool_redis::Connection,
    is_default: bool,
    amount: f64,
) -> Result<(), ServerError> {
    let timestamp: u64 = chrono::Utc::now().timestamp_millis().try_into()?;
    let key = if is_default { KEY_DEFAULT } else { KEY_FALLBACK };
    let entry = serde_json::to_string(&SummaryEntry { amount, timestamp })?;
    con.zadd::<&str,f64,String,usize>(key, entry, timestamp as f64).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

pub async fn get_summary(
    con: &mut deadpool_redis::Connection,
    from: Option<&String>,
    to: &str,
) -> Result<CheckRequest, ServerError> {
    let min = from
        .map(|f| parse_timestamp(f))
        .transpose()?
        .unwrap_or(f64::NEG_INFINITY);
    let max = parse_timestamp(to)?;

    Ok(CheckRequest {
        default: summarize_processor(con, KEY_DEFAULT, min, max).await?,
        fallback: summarize_processor(con, KEY_FALLBACK, min, max).await?,
    })
}

/// Retrieves a summary of payments processed by a given processor within the specified time range.
async fn summarize_processor(
    con: &mut deadpool_redis::Connection,
    key: &str,
    min: f64,
    max: f64,
) -> Result<ProcessorSummary, ServerError> {
    let min_str = if min == f64::NEG_INFINITY {
        "-inf".to_string()
    } else {
        min.to_string()
    };
    let max_str = if max == f64::INFINITY {
        "+inf".to_string()
    } else {
        max.to_string()
    };

    let entries: Vec<String> = deadpool_redis::redis::cmd("ZRANGEBYSCORE")
        .arg(key)
        .arg(&min_str)
        .arg(&max_str)
        .query_async(&mut **con)
        .await?;

    let total_requests = entries.len();
    let total_amount = entries
        .iter()
        .filter_map(|e| serde_json::from_str::<SummaryEntry>(e).ok())
        .map(|e| e.amount)
        .sum();

    Ok(ProcessorSummary {
        total_requests,
        total_amount,
    })
}

fn parse_timestamp(s: &str) -> Result<f64, ServerError> {
    Ok(DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp_millis() as f64)?)
}