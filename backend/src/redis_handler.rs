use redis::TypedCommands;
use crate::{errors::ServerError, jsons::{check_request::{CheckRequest, ProcessorSummary}, payment_request::PaymentRequest}};
use chrono::DateTime;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SummaryEntry{
    amount : f64,
    timestamp : u64 // in ms
}

pub const KEY_DEFAULT: &str = "payments:default";
pub const KEY_FALLBACK: &str = "payments:fallback";

pub const DEFAULT_IS_ALIVE: &str = "default_is_alive";
pub const FALLBACK_IS_ALIVE: &str = "fallback_is_alive";

pub const NOT_PROCESSED : &str = "not_processed_payments";

pub fn add_not_processed(
    con : &mut redis::Connection,
    payment : &PaymentRequest
) -> Result<(),ServerError>{
    con.lpush(NOT_PROCESSED, serde_json::to_string(payment)?)?;
    Ok(())
}
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
pub fn get_default_is_alive(con : &mut redis::Connection) -> Result<bool,ServerError>{
    let is_alive : bool = match con.get(DEFAULT_IS_ALIVE)?{
        Some(vl) => vl.parse()?,
        None => return Err(ServerError::RedisClientError("default_is_alive not found".to_string()))
    };
    Ok(is_alive)
}
pub fn get_fallback_is_alive(con : &mut redis::Connection) -> Result<bool,ServerError>{
    let is_alive : bool = match con.get(FALLBACK_IS_ALIVE)?{
        Some(vl) => vl.parse()?,
        None => return Err(ServerError::RedisClientError("fallback_is_alive not found".to_string()))
    };
    Ok(is_alive)
}

pub fn set_default_is_alive(con : &mut redis::Connection, is_alive : bool) -> Result<(),ServerError>{
    con.set(DEFAULT_IS_ALIVE, is_alive.to_string())?;
    Ok(())
}

pub fn set_fallback_is_alive(con : &mut redis::Connection, is_alive : bool) -> Result<(),ServerError>{
    con.set(FALLBACK_IS_ALIVE, is_alive.to_string())?;
    Ok(())
}


//-----------------------------------
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

pub fn get_summary(
    con: &mut redis::Connection,
    from: Option<&String>,
    to: &str,
) -> Result<CheckRequest, ServerError> {
    let min = from
        .map(|f| {
            DateTime::parse_from_rfc3339(f)
                .map(|dt| dt.timestamp_millis() as f64)
                .map_err(|e| ServerError::ParseError(e.to_string()))
        })
        .transpose()?
        .unwrap_or(f64::NEG_INFINITY);

    let max = DateTime::parse_from_rfc3339(to)
        .map(|dt| dt.timestamp_millis() as f64)
        .map_err(|e| ServerError::ParseError(e.to_string()))?;

    Ok(CheckRequest {
        default: summarize_processor(con, KEY_DEFAULT, min, max)?,
        fallback: summarize_processor(con, KEY_FALLBACK, min, max)?,
    })
}


//================================================================================

fn summarize_processor(
    con: &mut redis::Connection,
    processor: &str,
    min: f64,
    max: f64,
) -> Result<ProcessorSummary, ServerError> {
    let key = format!("payments:{}", processor);
    let entries: Vec<String> = con.zrangebyscore(key, min, max)?;

    let total_requests = entries.len();
    let total_amount = entries
        .iter()
        .filter_map(|e| serde_json::from_str::<SummaryEntry>(e).ok())
        .map(|e| e.amount)
        .sum();

    Ok(ProcessorSummary { total_requests, total_amount })
}