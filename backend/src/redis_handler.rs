use redis::TypedCommands;
use crate::{errors::ServerError, jsons::{check_request::{CheckRequest, ProcessorSummary}, payment_request::PaymentRequest}};
use chrono::DateTime;

/// Represents a payment entry stored for summary purposes.
///
/// # Fields
/// * `amount` - Payment value.
/// * `timestamp` - Time when the payment occurred, in milliseconds since epoch.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SummaryEntry{
    amount : f64,
    timestamp : u64 // in ms
}

/// Redis key used to store payments processed by the default processor.
pub const KEY_DEFAULT: &str = "payments:default";

/// Redis key used to store payments processed by the fallback processor.
pub const KEY_FALLBACK: &str = "payments:fallback";

/// Redis key indicating whether the default processor is alive.
pub const DEFAULT_IS_ALIVE: &str = "default_is_alive";

/// Redis key indicating whether the fallback processor is alive.
pub const FALLBACK_IS_ALIVE: &str = "fallback_is_alive";

/// Redis list key that stores payments that could not be processed.
pub const NOT_PROCESSED : &str = "not_processed_payments";



/// Adds a payment to the list of payments that were not processed in Redis.
///
/// # Parameters
///
/// * `con` - A connection to Redis.
/// * `payment` - The payment to add to the list of not processed payments.
///
/// # Return
///
/// A `Result` containing `Ok(())` if the payment was successfully added, and `Err(ServerError)` if there was an error.
pub fn add_not_processed(
    con : &mut redis::Connection,
    payment : &PaymentRequest
) -> Result<(),ServerError>{
    con.lpush(NOT_PROCESSED, serde_json::to_string(payment)?)?;
    Ok(())
}

/// Retrieves all the payments that were not processed from Redis
///
/// Returns a vector of `PaymentRequest` containing all the not processed payments
///
/// # Errors
///
/// Will return a `ServerError` if there was an error communicating with Redis
pub fn get_not_processed(
    con : &mut redis::Connection,
) -> Result<Vec<PaymentRequest>,ServerError>{
    let entries : Vec<String> = con.lrange(NOT_PROCESSED, 0, -1)?;
    con.del(NOT_PROCESSED)?;
    Ok(
        entries
        .iter()
        .map(|e|
            serde_json::from_str(e)
            .unwrap_or_default()
        ).collect()
    )
}

//-----------------------------------

/// Gets the health status of the default processor from Redis.
///
/// # Returns
/// * `true` if the processor is alive.
/// * `false` if the processor is not alive.
///
/// # Errors
/// Returns `ServerError` if the key is missing or Redis fails.
pub fn get_default_is_alive(con : &mut redis::Connection) -> Result<bool,ServerError>{
    let is_alive : bool = match con.get(DEFAULT_IS_ALIVE)?{
        Some(vl) => vl.parse()?,
        None => return Err(ServerError::RedisClientError("default_is_alive not found".to_string()))
    };
    Ok(is_alive)
}

/// Gets the health status of the fallback processor from Redis.
///
/// # Returns
/// * `true` if the processor is alive.
/// * `false` if the processor is not alive.
///
/// # Errors
/// Returns `ServerError` if the key is missing or Redis fails.
pub fn get_fallback_is_alive(con : &mut redis::Connection) -> Result<bool,ServerError>{
    let is_alive : bool = match con.get(FALLBACK_IS_ALIVE)?{
        Some(vl) => vl.parse()?,
        None => return Err(ServerError::RedisClientError("fallback_is_alive not found".to_string()))
    };
    Ok(is_alive)
}

/// Sets the health status of the default processor in Redis.
///
/// # Parameters
/// * `con` - Redis connection.
/// * `is_alive` - Processor health status.
pub fn set_default_is_alive(con : &mut redis::Connection, is_alive : bool) -> Result<(),ServerError>{
    con.set(DEFAULT_IS_ALIVE, is_alive.to_string())?;
    Ok(())
}

/// Sets the health status of the fallback processor in Redis.
///
/// # Parameters
/// * `con` - Redis connection.
/// * `is_alive` - Processor health status.
pub fn set_fallback_is_alive(con : &mut redis::Connection, is_alive : bool) -> Result<(),ServerError>{
    con.set(FALLBACK_IS_ALIVE, is_alive.to_string())?;
    Ok(())
}


//-----------------------------------

/// Stores a processed payment in Redis for summary purposes.
///
/// Payments are stored in a sorted set using the timestamp as the score.
///
/// # Parameters
/// * `con` - Redis connection.
/// * `is_df` - `true` for default processor, `false` for fallback.
/// * `amount` - Payment amount.
pub fn add_payment(
    con: &mut redis::Connection,
    is_df: bool, //is default or fallback
    amount: f64,
) -> Result<(), ServerError> {
    let timestamp : u64 = chrono::Utc::now().timestamp_millis().try_into()?;
    let key = format!("payments:{}", if is_df {"default"} else {"fallback"});
    let entry = serde_json::to_string(&SummaryEntry { amount, timestamp })
        .map_err(|e| ServerError::RedisClientError(e.to_string()))?;

    con.zadd(key, entry, timestamp as f64)?;
    Ok(())
}

//-----------------------------------

/// Generates a payment summary within a time range.
///
/// # Parameters
/// * `con` - Redis connection.
/// * `from` - Optional start timestamp (RFC3339 format).
/// * `to` - End timestamp (RFC3339 format).
///
/// # Returns
/// A `CheckRequest` containing summaries for both processors.
pub fn get_summary(
    con: &mut redis::Connection,
    from: Option<&String>,
    to: &str,
) -> Result<CheckRequest, ServerError> {
    let min = from
        .map(|f| parse_timestamp(f))
        .transpose()?
        .unwrap_or(f64::NEG_INFINITY);
    let max = parse_timestamp(to)?;

    Ok(CheckRequest {
        default: summarize_processor(con, KEY_DEFAULT, min, max)?,
        fallback: summarize_processor(con, KEY_FALLBACK, min, max)?,
    })
}

/// Calculates the summary for a specific processor.
///
/// # Parameters
/// * `con` - Redis connection.
/// * `key` - Redis sorted set key.
/// * `min` - Minimum timestamp score.
/// * `max` - Maximum timestamp score.
///
/// # Returns
/// `ProcessorSummary` with total requests and total amount.
fn summarize_processor(
    con: &mut redis::Connection,
    key: &str,
    min: f64,
    max: f64,
) -> Result<ProcessorSummary, ServerError> {
    // Converte f64 para string que o Redis entende
    let min_str = if min == f64::NEG_INFINITY { "-inf".to_string() } else { min.to_string() };
    let max_str = if max == f64::INFINITY { "+inf".to_string() } else { max.to_string() };

    let entries: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(&key)
        .arg(&min_str)
        .arg(&max_str)
        .query(con)?;

    let total_requests = entries.len();
    let total_amount = entries
        .iter()
        .filter_map(|e| serde_json::from_str::<SummaryEntry>(e).ok())
        .map(|e| e.amount)
        .sum();

    Ok(ProcessorSummary { total_requests, total_amount })
}

/// Parses an RFC3339 timestamp string into a millisecond timestamp.
///
/// # Parameters
/// * `s` - Timestamp string (RFC3339 format).
///
/// # Returns
/// Timestamp as `f64` representing milliseconds since epoch.
fn parse_timestamp(s: &str) -> Result<f64, ServerError> {
    Ok(DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp_millis() as f64)?)
}