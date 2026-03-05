use std::{collections::VecDeque, sync::Arc};

use redis::Commands;
use tokio::sync::{Mutex};
use crate::{app_state, errors::ServerError, jsons::{check_request::CheckRequest, payment_request::PaymentRequest}, redis_handler};

pub enum PaymentProcessor{
    Default,
    Fallback,
    Error,
}

#[derive(Clone)]
pub struct Server{
    pub address: String,
    pub is_alive: Arc<Mutex<bool>>,
    pub weight: usize,
    pub p_request_queue: Arc<Mutex<VecDeque<PaymentRequest>>>,
    pub c_request_queue: Arc<Mutex<VecDeque<CheckRequest>>>,
}


impl Server{
    pub async fn new(address: String, weight: usize) -> Server{
        Server{
            address,
            is_alive: Arc::new(Mutex::new(true)),
            weight,
            p_request_queue: Arc::new(Mutex::new(VecDeque::new())),
            c_request_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub async fn check(&self) -> bool{
        *self.is_alive.lock().await
    }

    pub fn is_max_weight(&self) -> bool{
        self.weight >= 10000
    }

    pub async fn add_p_request(&self, p_req : PaymentRequest){
        let mut p_req_qe  = self.p_request_queue.lock().await;
        match p_req_qe.try_reserve(1){
            Ok(_) => {
                p_req_qe.push_back(p_req);
            },
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    pub async fn add_c_request(&self, c_req : CheckRequest) -> Result<(),ServerError>{
        let mut c_req_qe  = self.c_request_queue.lock().await;
        match c_req_qe.try_reserve(1){
            Ok(_) => {
                c_req_qe.push_back(c_req);
                Ok(())
            },
            Err(err) => {
                Err(ServerError::MemoryNotAllocated(err.to_string()))
            }
        }
    }

    pub async fn process_p_request(
        &self,
        app_state : Arc<Mutex<app_state::AppState>>
    ) -> Result<(),ServerError>{
        
        let p_req = match self.p_request_queue.lock().await.pop_front(){
            Some(p_req) => p_req,
            None => return Ok(()),
        };
        let queue = self.p_request_queue.clone();

        tokio::spawn(async move{
            let rs = handle_request(app_state.clone(), p_req.clone()).await;
            let mut success : bool = true;

            match rs{
                Ok(payment_processor) => {
                    match payment_processor{
                        PaymentProcessor::Default => {

                        },
                        PaymentProcessor::Fallback => {

                        },
                        PaymentProcessor::Error => success = false,
                    }
                },
                Err(_) => success = false,
            }
            if !success{
                let mut qe = queue.lock().await;
                    
                match qe.try_reserve(1){
                    Ok(_) => {
                        qe.push_back(p_req);
                    },
                    Err(err) => {
                        ServerError::BalancerQueueError(err.to_string());
                    }
                };
            }
        });
        
        Ok(())
    }
}



pub async fn handle_request(
    app_state : Arc<Mutex<app_state::AppState>>,
    p_req : PaymentRequest
) -> Result<PaymentProcessor,ServerError>{
    let (default_addr, fallback_addr, default_client, fallback_client, redis_cli) = {
        let app = app_state.lock().await;
        (
            app.default_address.clone(),
            app.fallback_address.clone(),
            app.default_client.clone(),
            app.fallback_client.clone(),
            app.redis_client.clone(),
        )
    };

    let default_response = 
        default_client
        .post(default_addr.clone())
        .json(&p_req)
        .send()
        .await?;


    if default_response.status().is_success(){
        redis_handler::inc_df_payments(&redis_cli).await?;
        return Ok(PaymentProcessor::Default);
    }

    let fallback_response = 
        fallback_client
        .post(fallback_addr.clone())
        .json(&p_req)
        .send()
        .await?;

    if fallback_response.status().is_success(){
        redis_handler::inc_fb_payments(&redis_cli).await?;
        return Ok(PaymentProcessor::Fallback);
    }

    Ok(PaymentProcessor::Error)
}
