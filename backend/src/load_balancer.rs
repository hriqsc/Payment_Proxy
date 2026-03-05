use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{errors::ServerError, server::Server};


pub struct LoadBalancer{
    servers : Arc<Mutex<Vec<Server>>>,
    round_robin_count: usize,
}


impl LoadBalancer{
    pub async fn new() -> LoadBalancer{
        LoadBalancer{
            servers: Arc::new(Mutex::new(Vec::new())),
            round_robin_count : 0,
        }
    }
    pub async fn select_server(&mut self) -> Result<Option<Server>, ServerError>
    {
        let servers = self.servers.lock().await;
        let len = servers.len();

        if len == 0 {
            return Ok(None);
        }

        for _ in 0..len {
            self.round_robin_count = (self.round_robin_count + 1) % len;
            let idx = self.round_robin_count;

            if servers[idx].check().await {
                return Ok(Some(servers[idx].clone()));
            }
        }

        Ok(None)
    }


    pub async fn add_server(&mut self, server: Server) -> Option<ServerError>{
        match self.servers.lock().await.try_reserve(1){
            Ok(_) => {
                self.servers.lock().await.push(server);
            },
            Err(err) => return Some(ServerError::BalancerAddSVError(err.to_string()))           
        };
        None
    }

    pub async fn remove_server(&mut self, address: String) -> Result<(),ServerError>{
        let mut svs = self.servers.lock().await;

        match svs.iter().position(|sv| sv.address == address){
            None => return Err(ServerError::BalancerAddSVError("Server not found".to_string())),
            Some(i) => {
                svs.remove(i);
               Ok(())
            }
        }
    }


    
}

