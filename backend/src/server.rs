use std::sync::Arc;
use tokio::sync::{Mutex};
use crate::{app_state, errors::ServerError, jsons::payment_request::PaymentRequest, redis_handler};

#[derive(Clone)]
pub struct Server{
    pub id: usize,
    pub is_alive: Arc<Mutex<bool>>,
    pub weight: usize,
}

pub const PAYMENT_ENDPOINT: &str = "/payment";

impl Server{
    pub async fn new(id: usize, weight: usize) -> Server{
        Server{
            id,
            is_alive: Arc::new(Mutex::new(true)),
            weight,
        }
    }
    pub async fn check(&self) -> bool{
        *self.is_alive.lock().await
    }

    pub fn is_max_weight(&self) -> bool{
        self.weight >= 10000
    }


    pub async fn process_request(
        &mut self,
        app_state : Arc<Mutex<app_state::AppState>>,
        p_req : PaymentRequest
    ) -> Result<(),ServerError>{
        let (default_addr, fallback_addr,sv_client,mut redis_con) = {
            let app = app_state.lock().await;
            (
                app.default_address.clone(),
                app.fallback_address.clone(),
                app.server_client.clone(),
                app.redis_client.clone().get_connection()?
            )
        };

        self.weight += 1;

        let rs = handle_request(
            p_req.clone(),
            &default_addr,
            &fallback_addr,
            &sv_client,
            &mut redis_con
        ).await;

        self.weight -= 1;

        if let Err(err) = rs{
            redis_handler::add_not_processed(&mut redis_con, &p_req)?;
            return Err(err)
        }

        Ok(())
    }
}


pub async fn handle_request(
    p_req : PaymentRequest,
    default_addr : &str,
    fallback_addr : &str,
    sv_client : &reqwest::Client,
    redis_con : &mut redis::Connection
) -> Result<(),ServerError>{
    
    //tries to send to default processor
    let df_is_alive = redis_handler::get_default_is_alive(redis_con)?;
    if df_is_alive{
        let req_builder = sv_client
            .post(format!("{}{}", default_addr, PAYMENT_ENDPOINT))
            .body(serde_json::to_string(&p_req)?);

        //attempts to send 3 times
        for _ in 0..3{
            let restult = match req_builder.try_clone(){
                Some(rb) => rb.send().await?,
                None => return Err(ServerError::ReqErrorGeneric("Failed to clone request builder".to_string()))
            };
            if restult.status().is_success(){
                redis_handler::add_payment(redis_con, true, p_req.amount)?;
                return Ok(());
            }else{
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
        }
    }

    //if default could not process, tries to send to fallback
    let fb_is_alive = redis_handler::get_fallback_is_alive(redis_con)?;
    if fb_is_alive{
        let restult = sv_client.post(format!("{}{}", fallback_addr, PAYMENT_ENDPOINT))
        .body(serde_json::to_string(&p_req)?)
        .send()
        .await?;

        if restult.status().is_success(){
            redis_handler::add_payment(redis_con, false, p_req.amount)?;
            return Ok(());
        }
        else{
            return Err(ServerError::ReqErrorGeneric(restult.status().to_string()));
        }
    }

    Err(ServerError::BalancerQueueError("Was unable to process request".to_string()))
}