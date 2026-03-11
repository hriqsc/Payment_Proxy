use std::{env::VarError, num::TryFromIntError, str::ParseBoolError, sync::PoisonError};

use redis::RedisError;
use tokio::task::JoinError;

/// Central error type used across the server, load balancer, and infrastructure layers.
pub enum ServerError{
    //common errors

    /// Generic mutex error, usually caused by poisoned locks.
    MutexError(String),

    /// Generic fallback error for unexpected conditions.
    OtherError(String),

    /// Error indicating that memory allocation failed.
    MemoryNotAllocated(String),

    /// Environment variable access or parsing error.
    EnvVarError(String),

    /// Generic parsing error.
    ParseError(String),
    
    //server errors

    /// Indicates that a server is not alive or reachable.
    ServerNotAlive(String),

    //load balancer

    /// Mutex error related to load balancer server storage.
    BalancerMutexError(String),

    /// Error when selecting a server in the load balancer.
    SelectingServerError(String),

    /// Error when adding a server to the load balancer.
    BalancerAddSVError(String),

    /// Error when removing a server from the load balancer.
    BalancerRemoveSVError(String),

    /// Error when retrieving a server from the load balancer.
    BalancerGetSVError(String),

    /// Error related to request queueing in the load balancer.
    BalancerQueueError(String),

    /// Error indicating that the load balancer has no servers available.
    BalancerEmptyServersError(String),

    //request error

    /// Generic HTTP request error.
    ReqErrorGeneric(String),

    //axum

    /// General error originating from the Axum framework.
    AxumCommonErr(String),

    /// IO-related error from Axum or underlying system operations.
    AxumIOError(String),
    
    //redis

    /// Error from the Redis client abstraction.
    RedisClientError(String),

    /// Low-level Redis communication error.
    RedisError(String),

    /// JSON serialization or deserialization error.
    JSONError(String),

    /// Database-related error.
    DatabaseError(String),

}


/// Implements the `Display` trait for `ServerError`,
/// providing human-readable error messages.
impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ServerError::DatabaseError(err)             => format!("Database Error: {}", err),
            ServerError::OtherError(err)                => format!("Other Error: {}", err),
            ServerError::MutexError(err)                => format!("Mutex Error: {}", err),
            ServerError::JSONError(err)                 => format!("JSON Error: {}", err),
            ServerError::ServerNotAlive(err)            => format!("Server Not Alive: {}", err),
            ServerError::SelectingServerError(err)      => format!("Selecting Server Error: {}", err),
            ServerError::BalancerMutexError(err)        => format!("Balancer Servers Mutex Error: {}", err),
            ServerError::BalancerAddSVError(err)        => format!("Balancer Add Server Error: {}", err),
            ServerError::BalancerRemoveSVError(err)     => format!("Balancer Remove Server Error: {}", err),
            ServerError::BalancerGetSVError(err)        => format!("Balancer Get Server Error: {}", err),
            ServerError::BalancerQueueError(err)        => format!("Balancer Queue Error: {}", err),
            ServerError::BalancerEmptyServersError(err) => format!("Balancer Empty Servers Error: {}", err),
            ServerError::ReqErrorGeneric(err)           => format!("Request Error: {}", err),
            ServerError::MemoryNotAllocated(err)        => format!("Memory Not Allocated: {}", err),
            ServerError::AxumCommonErr(err)             => format!("Axum Common Error: {}", err),
            ServerError::AxumIOError(err)               => format!("Axum IO Error: {}", err),
            ServerError::RedisClientError(err)          => format!("Redis Client Error: {}", err),
            ServerError::RedisError(err)                => format!("Redis Error: {}", err),
            ServerError::EnvVarError(err)               => format!("Env Var Error: {}", err),
            ServerError::ParseError(err)                => format!("Parse Error: {}", err),
        })
    }
}

/// Converts `VarError` (environment variable errors) into `ServerError`.
impl From<VarError> for ServerError {
    fn from(err: VarError) -> Self {
        ServerError::EnvVarError(err.to_string())
    }
}

/// Converts Redis client errors into `ServerError`.
impl From<RedisError> for ServerError {
    fn from(err: RedisError) -> Self {
        ServerError::RedisError(err.to_string())
    }
}

/// Converts poisoned mutex errors into `ServerError`.
impl<T> From<PoisonError<T>> for ServerError {
    fn from(err: PoisonError<T>) -> Self {
        ServerError::MutexError(err.to_string())
    }
}

/// Converts HTTP request errors (`reqwest`) into `ServerError`.
impl From<reqwest::Error> for ServerError {
    fn from(err: reqwest::Error) -> Self {
        ServerError::ReqErrorGeneric(err.to_string())
    }
}

/// Converts Axum framework errors into `ServerError`.
impl From<axum::Error> for ServerError {
    fn from(err: axum::Error) -> Self {
        ServerError::AxumCommonErr(err.to_string())
    }
}

/// Converts standard IO errors into `ServerError`.
impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        ServerError::AxumIOError(err.to_string())
    }
}

/// Converts integer conversion errors into `ServerError`.
impl From<TryFromIntError> for ServerError {
    fn from(err: TryFromIntError) -> Self {
        ServerError::ParseError(err.to_string())
    }
}

/// Converts boolean parsing errors into `ServerError`.
impl From<ParseBoolError> for ServerError {
    fn from(err: ParseBoolError) -> Self {
        ServerError::ParseError(err.to_string())
    }
}

/// Converts JSON serialization/deserialization errors into `ServerError`.
impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> Self {
        ServerError::JSONError(err.to_string())
    }
}

/// Converts asynchronous task join errors into `ServerError`.
impl From<JoinError> for ServerError {
    fn from(err: JoinError) -> Self {
        ServerError::ReqErrorGeneric(err.to_string())
    }
}

/// Converts RFC3339 timestamp parsing errors into `ServerError`.
impl From<chrono::ParseError> for ServerError {
    fn from(err: chrono::ParseError) -> Self {
        ServerError::ParseError(err.to_string())
    }
}