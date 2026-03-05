use std::sync::{Arc, Mutex};
use tokio_threadpool::ThreadPool;

use crate::errors::ServerError;

#[derive(Clone)]
pub struct Server{
    pub address: String,
    pub is_alive: Arc<Mutex<bool>>,
    pub weight: usize,
    pub thread_pool: Arc<Mutex<ThreadPool>>,
}



impl Server{
    pub async fn new(address: String, weight: usize) -> Server{
        Server{
            address,
            is_alive: Arc::new(Mutex::new(true)),
            weight,
            thread_pool: Arc::new(Mutex::new(ThreadPool::new())),
        }
    }
    pub async fn check(&self) -> Result<bool,ServerError>{
        match self.is_alive.lock() {
            Ok(lock) => Ok(*lock),
            Err(err) => Err(ServerError::MutexError(err.to_string()))
        }
    }
}