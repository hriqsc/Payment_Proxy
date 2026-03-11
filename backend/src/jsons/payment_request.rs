use serde::{Deserialize, Serialize};



/// Represents a payment request received by the API.
///
/// This structure contains the basic information required
/// to process a payment through the load balancer.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PaymentRequest {

    /// Payment amount to be processed.
    pub amount: f64,

    /// Unique identifier used to correlate the payment request.
    #[serde(rename = "correlationId")]
    pub id : String,
}

/// Structure sent to a payment processor.
///
/// This is derived from `PaymentRequest` but includes
/// the timestamp indicating when the request was issued.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ProcessorPRequest{

    /// Payment amount to be processed.
    pub amount: f64,

    /// Unique identifier used to correlate the payment request.
    #[serde(rename = "correlationId")]
    pub id : String,

    /// Timestamp (RFC3339) indicating when the request was created.
    #[serde(rename = "requestedAt")]
    pub requested_at : String
}



/*
https://github.com/zanfranceschi/rinha-de-backend-2025/blob/main/INSTRUCOES.md#payments

POST /payments
{
    "correlationId": "4a7901b8-7d26-4d9d-aa19-4dc1c7cf60b3",
    "amount": 19.90
}

HTTP 2XX
Qualquer coisa

*/