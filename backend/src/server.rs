use std::sync::Arc;
use chrono::{SecondsFormat, Utc};
use tokio::sync::{Mutex};
use crate::{app_state, errors::ServerError, jsons::payment_request::{PaymentRequest, ProcessorPRequest}, redis_handler};

/// Represents a backend processor server used by the load balancer.
///
/// # Fields
/// * `id` - Unique identifier of the server.
/// * `is_alive` - Shared flag indicating whether the server is alive.
/// * `weight` - Current load weight (number of active requests).
#[derive(Clone)]
pub struct Server{
    pub id: usize,
    pub is_alive: Arc<Mutex<bool>>,
    pub weight: usize,
}

/// Endpoint used to send payment requests to processors.
pub const PAYMENT_ENDPOINT: &str = "/payments";

impl Server{

    /// Creates a new `Server` instance.
    ///
    /// # Parameters
    /// * `id` - Unique identifier of the server.
    /// * `weight` - Initial load weight.
    pub async fn new(id: usize, weight: usize) -> Server{
        Server{
            id,
            is_alive: Arc::new(Mutex::new(true)),
            weight,
        }
    }

    /// Checks whether the server is currently marked as alive.
    ///
    /// # Returns
    /// `true` if the server is alive, otherwise `false`.
    pub async fn check(&self) -> bool{
        *self.is_alive.lock().await
    }

    /// Verifies if the server reached the maximum allowed weight.
    ///
    /// # Returns
    /// `true` if the weight is greater than or equal to `10000`.
    pub fn is_max_weight(&self) -> bool{
        self.weight >= 10000
    }


    /// Processes a payment request using either the default or fallback processor.
    ///
    /// # Parameters
    /// * `app_state` - Shared application state.
    /// * `p_req` - Payment request to process.
    ///
    /// # Behavior
    /// * Increments the server weight while the request is being processed.
    /// * Attempts to process the request using `handle_request`.
    /// * If processing fails, the request is stored in Redis for later retry.
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

        if rs.is_err(){
            redis_handler::add_not_processed(&mut redis_con, &p_req)?;
        }

        Ok(())
    }
}


/// Sends a payment request to the processors.
///
/// # Behavior
/// 1. Attempts to send the request to the default processor (up to 3 retries).
/// 2. If it fails or the default processor is down, attempts the fallback processor.
/// 3. If both fail, returns an error.
///
/// # Parameters
/// * `p_req` - Payment request.
/// * `default_addr` - Address of the default processor.
/// * `fallback_addr` - Address of the fallback processor.
/// * `sv_client` - HTTP client used to send requests.
/// * `redis_con` - Redis connection used for health checks and logging payments.
pub async fn handle_request(
    p_req : PaymentRequest,
    default_addr : &str,
    fallback_addr : &str,
    sv_client : &reqwest::Client,
    redis_con : &mut redis::Connection
) -> Result<(),ServerError>{

    let request = ProcessorPRequest{
        amount: p_req.amount,
        requested_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        id: p_req.id,
    };

    //tries to send to default processor
    let df_is_alive = redis_handler::get_default_is_alive(redis_con)?;
    if df_is_alive{
        let req_builder = sv_client
            .post(format!("{}{}", default_addr, PAYMENT_ENDPOINT))
            .json(&request);

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
        .json(&request)
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