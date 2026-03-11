use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{errors::ServerError, load_balancer::LoadBalancer};

/// Shared application state used across request handlers and services.
///
/// This structure stores global resources such as HTTP clients,
/// processor addresses, the load balancer, and the Redis client.
pub struct AppState{

    /// HTTP client used to send requests to payment processors.
    pub server_client : reqwest::Client,

    /// Address of the default payment processor.
    pub default_address : String,

    /// Address of the fallback payment processor.
    pub fallback_address : String,

    /// Load balancer responsible for distributing requests across servers.
    pub load_balancer : Arc<Mutex<LoadBalancer>>,

    /// Redis client used for caching and persistence.
    pub redis_client : redis::Client,
}


impl AppState{

    /// Creates a new `AppState` instance.
    ///
    /// # Parameters
    /// * `default_address` - URL of the default payment processor.
    /// * `fallback_address` - URL of the fallback payment processor.
    /// * `load_balancer` - Shared load balancer instance.
    /// * `redis_address` - Redis connection string.
    ///
    /// # Errors
    /// Returns `ServerError` if the Redis client cannot be created.
    pub fn new(
        default_address: String,
        fallback_address: String,
        load_balancer: Arc<Mutex<LoadBalancer>>,
        redis_address: String
    ) -> Result<AppState, ServerError>{
        Ok(
            AppState { 
                server_client : reqwest::Client::new(),
                default_address,
                fallback_address,
                load_balancer,
                redis_client : redis::Client::open(redis_address)?,
            }
        )
    }

    
}