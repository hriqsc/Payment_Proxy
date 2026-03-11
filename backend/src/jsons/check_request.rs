use serde::{Deserialize, Serialize};

/// Response structure used to return a summary of processed payments.
///
/// It contains aggregated information for both the default and fallback processors.
#[derive(Debug,Deserialize, Serialize,Default)]
pub struct CheckRequest{

    /// Summary of payments processed by the default processor.
    pub default : ProcessorSummary,

    /// Summary of payments processed by the fallback processor.
    pub fallback: ProcessorSummary,
}

/// Aggregated statistics for a specific payment processor.
#[derive(Debug,Serialize,Deserialize,Default)]
pub struct ProcessorSummary {

    /// Total number of processed requests.
    #[serde(rename = "totalRequests")]
    pub total_requests: usize,

    /// Sum of all processed payment amounts.
    #[serde(rename = "totalAmount")]
    pub total_amount: f64,
}


/*

https://github.com/zanfranceschi/rinha-de-backend-2025/blob/main/INSTRUCOES.md#payments

HTTP 200 - Ok
{
    "default" : {
        "totalRequests": 43236,
        "totalAmount": 415542345.98
    },
    "fallback" : {
        "totalRequests": 423545,
        "totalAmount": 329347.34
    }
}


*/