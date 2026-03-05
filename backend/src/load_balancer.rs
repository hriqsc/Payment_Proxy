use std::sync::{Arc, Mutex};
use crate::{errors::ServerError, server::{self, Server}};
use std::collections::VecDeque;
use crate::jsons::{payment_request::PaymentRequest, check_request::CheckRequest};


pub struct LoadBalancer{
    servers : Arc<Mutex<Vec<Server>>>, //i could use a hashmap, but it cost more to the memory than a vector
    round_robin_count: usize,
    p_request_queue: Arc<Mutex<VecDeque<PaymentRequest>>>,
    c_request_queue: Arc<Mutex<VecDeque<CheckRequest>>>,
}


impl LoadBalancer{
    pub async fn new() -> LoadBalancer{
        LoadBalancer{
            servers: Arc::new(Mutex::new(Vec::new())),
            round_robin_count : 0,
            p_request_queue: Arc::new(Mutex::new(VecDeque::new())),
            c_request_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }


    pub async fn select_server<F, T>(&mut self, f: F) -> Result<Option<T>, ServerError>
    where
        F: FnOnce(&mut Server) -> T,
    {
        let mut servers = match self.servers.lock() {
            Ok(svs) => svs,
            Err(err) => return Err(ServerError::BalancerMutexError(err.to_string())),
        };

        let len = servers.len();

        if len == 0 {
            return Ok(None);
        }

        for _ in 0..len {
            self.round_robin_count = (self.round_robin_count + 1) % len;
            let idx = self.round_robin_count;

            let is_alive = match servers[idx].check().await {
                Ok(alive) => alive,
                Err(err) => return Err(ServerError::SelectingServerError(err.to_string())),
            };

            if is_alive {
                return Ok(Some(f(&mut servers[idx])));
            }
        }

        Ok(None)
    }

    pub fn add_server(&mut self, server: Server) -> Option<ServerError>{
        match self.servers.lock(){
            Ok(mut svs) => {
                if let Err(err) = svs.try_reserve(1){
                    return Some(ServerError::BalancerAddSVError(err.to_string()))
                }
                svs.push(server);
            },
            Err(err) => {
                return Some(ServerError::BalancerAddSVError(err.to_string()))
            }
        };
        None
    }
    pub fn remove_server(&mut self, address: String) -> Option<ServerError>{
        match self.servers.lock(){
            Ok(mut svs) => {
                match svs.binary_search_by(|sv| sv.address.as_str().cmp(address.as_str())) {
                    Ok(idx) => svs.remove(idx),
                    Err(err) => {
                        return Some(ServerError::BalancerAddSVError(err.to_string()))
                    }
                }
            },
            Err(err) => {
                return Some(ServerError::BalancerAddSVError(err.to_string()))
            }
        };
        None
    }

    pub fn add_p_request(&mut self, p_request: PaymentRequest) -> Option<ServerError>{
        match self.p_request_queue.lock(){
            Ok(mut p_reqs) => {
                if let Err(err) = p_reqs.try_reserve(1){
                    return Some(ServerError::BalancerQueueError(err.to_string()))
                }
                p_reqs.push_back(p_request);
            },
            Err(err) => {
                return Some(ServerError::BalancerQueueError(err.to_string()))
            }
        };
        None
    }

    pub fn add_c_request(&mut self, c_request: CheckRequest) -> Option<ServerError>{
        match self.c_request_queue.lock(){
            Ok(mut c_reqs) => {
                if let Err(err) = c_reqs.try_reserve(1){
                    return Some(ServerError::BalancerQueueError(err.to_string()))
                }
                c_reqs.push_back(c_request);
            },
            Err(err) => {
                return Some(ServerError::BalancerQueueError(err.to_string()))
            }
        };
        None
    }

    
}