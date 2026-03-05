use std::sync::PoisonError;

pub enum ServerError{
    //common errors
    MutexError(String),
    OtherError(String),
    
    //server errors
    ServerNotAlive(String),

    //load balancer
    BalancerMutexError(String),
    SelectingServerError(String),
    BalancerAddSVError(String),
    BalancerQueueError(String),

    JSONError(String),
    DatabaseError(String),
}


impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ServerError::DatabaseError(err)         => format!("Database Error: {}", err),
            ServerError::OtherError(err)            => format!("Other Error: {}", err),
            ServerError::MutexError(err)            => format!("Mutex Error: {}", err),
            ServerError::JSONError(err)             => format!("JSON Error: {}", err),
            ServerError::ServerNotAlive(err)        => format!("Server Not Alive: {}", err),
            ServerError::SelectingServerError(err)  => format!("Selecting Server Error: {}", err),
            ServerError::BalancerMutexError(err)    => format!("Balancer Servers Mutex Error: {}", err),
            ServerError::BalancerAddSVError(err)    => format!("Balancer Add Server Error: {}", err),
            ServerError::BalancerQueueError(err)    => format!("Balancer Queue Error: {}", err),
        })
    }
}



impl<T> From<PoisonError<T>> for ServerError {
    fn from(err: PoisonError<T>) -> Self {
        ServerError::MutexError(err.to_string())
    }
}