use std::sync::Arc;
use redis::TypedCommands;
use tokio::sync::Mutex;
use crate::{errors::ServerError, load_balancer::LoadBalancer};

pub struct AppState{
    pub default_client : reqwest::Client,
    pub fallback_client : reqwest::Client,
    pub default_address : String,
    pub fallback_address : String,
    pub load_balancer : Arc<Mutex<LoadBalancer>>,
    pub redis_client : redis::Client,
}


impl AppState{
    pub fn new(default_address: String, fallback_address: String, load_balancer: Arc<Mutex<LoadBalancer>>, redis_address: String) -> Result<AppState, ServerError>{
        let redis_client = redis::Client::open(redis_address)?;
        let mut con = redis_client.get_connection()?;
        con.set("default_payments",0)?;
        con.set("fallback_payments",0)?;

        Ok(
            AppState { 
                default_client : reqwest::Client::new(),
                fallback_client : reqwest::Client::new(),
                default_address,
                fallback_address,
                load_balancer,
                redis_client,
            }
        )
    }

    
}


