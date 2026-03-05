use std::sync::PoisonError;

pub enum ServerError{
    //common errors
    MutexError(String),
    OtherError(String),
    MemoryNotAllocated(String),
    
    //server errors
    ServerNotAlive(String),

    //load balancer
    BalancerMutexError(String),
    SelectingServerError(String),
    BalancerAddSVError(String),
    BalancerQueueError(String),
    BalancerEmptyServersError(String),

    //request error
    ReqErrorGeneric(String),

    //axum
    AxumCommonErr(String),
    AxumIOError(String),
    
    //redis
    RedisClientError(String),

    JSONError(String),
    DatabaseError(String),

}


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
            ServerError::BalancerQueueError(err)        => format!("Balancer Queue Error: {}", err),
            ServerError::BalancerEmptyServersError(err) => format!("Balancer Empty Servers Error: {}", err),
            ServerError::ReqErrorGeneric(err)           => format!("Request Error: {}", err),
            ServerError::MemoryNotAllocated(err)        => format!("Memory Not Allocated: {}", err),
            ServerError::AxumCommonErr(err)             => format!("Axum Common Error: {}", err),
            ServerError::AxumIOError(err)               => format!("Axum IO Error: {}", err),
            ServerError::RedisClientError(err)          => format!("Redis Client Error: {}", err),
        })
    }
}



impl<T> From<PoisonError<T>> for ServerError {
    fn from(err: PoisonError<T>) -> Self {
        ServerError::MutexError(err.to_string())
    }
}


impl From<reqwest::Error> for ServerError {
    fn from(err: reqwest::Error) -> Self {
        ServerError::ReqErrorGeneric(err.to_string())
    }
}


impl From<axum::Error> for ServerError {
    fn from(err: axum::Error) -> Self {
        ServerError::AxumCommonErr(err.to_string())
    }
}

impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        ServerError::AxumIOError(err.to_string())
    }
}

impl From<redis::RedisError> for ServerError {
    fn from(err: redis::RedisError) -> Self {
        ServerError::RedisClientError(err.to_string())
    }
    
}