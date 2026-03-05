use serde::{Deserialize, Serialize};


#[derive(Debug,Deserialize, Serialize)]
pub struct PaymentRequest {
    pub amount: f64,
    pub currency: String,
    pub description: String,
}