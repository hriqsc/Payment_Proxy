use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{errors::ServerError, server::Server};

/// Represents a simple load balancer responsible for managing servers
/// and distributing requests between them.
pub struct LoadBalancer{
    /// List of servers managed by the load balancer.
    servers : Arc<Mutex<Vec<Server>>>,
    
    /// Counter used to implement round-robin server selection.
    round_robin_count: usize,
}


impl LoadBalancer{

    /// Creates a new `LoadBalancer` instance with an empty server list.
    pub async fn new() -> LoadBalancer{
        LoadBalancer{
            servers: Arc::new(Mutex::new(Vec::new())),
            round_robin_count : 0,
        }
    }

    /// Selects a server using a round-robin strategy.
    ///
    /// # Behavior
    /// * If no servers exist, a new server is created automatically.
    /// * Iterates through the server list until it finds one that is alive.
    ///
    /// # Returns
    /// * `Some(Server)` if an available server is found.
    /// * `None` if all servers are unavailable.
    pub async fn select_server(&mut self) -> Result<Option<Server>, ServerError> {
        let len = {
            let servers = self.servers.lock().await;
            servers.len()
        };

        if len == 0 {
            let id = self.add_server().await?;
            return Ok(Some(self.get_server(id).await?));
        }

        let servers = self.servers.lock().await;
        for _ in 0..len {
            self.round_robin_count = (self.round_robin_count + 1) % len;
            let idx = self.round_robin_count;
            if servers[idx].check().await {
                return Ok(Some(servers[idx].clone()));
            }
        }
        Ok(None)
    }

    /// Retrieves a server by its identifier.
    ///
    /// # Parameters
    /// * `id` - Index of the server in the internal list.
    ///
    /// # Errors
    /// Returns an error if the server does not exist.
    pub async fn get_server(&mut self, id: usize) -> Result<Server, ServerError> {
        let servers = self.servers.lock().await;
        if id >= servers.len() {
            return Err(ServerError::BalancerGetSVError("Server does not exist".to_string()));
        }
        Ok(servers[id].clone())
    }


    /// Adds a new server to the load balancer.
    ///
    /// # Returns
    /// The identifier of the newly created server.
    ///
    /// # Errors
    /// Returns an error if memory allocation fails.
    pub async fn add_server(&mut self) -> Result<usize, ServerError> {
        let mut servers = self.servers.lock().await;
        servers.try_reserve(1)
            .map_err(|err| ServerError::BalancerAddSVError(err.to_string()))?;
        let id = servers.len();
        let sv = Server::new(id, 10000).await;
        servers.push(sv);
        Ok(id)
    }

    /// Removes a server from the load balancer.
    ///
    /// # Parameters
    /// * `id` - Identifier of the server to remove.
    ///
    /// # Errors
    /// Returns an error if the server does not exist.
    pub async fn remove_server(&mut self,id : usize) -> Result<(),ServerError>{
        let mut svs = self.servers.lock().await;

        if id >= svs.len(){
            return Err(ServerError::BalancerRemoveSVError("Server does not exist".to_string()));
        }
        svs.remove(id);
        Ok(())
    }
}