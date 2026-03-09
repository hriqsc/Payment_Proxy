use serde::{Deserialize, Serialize};

#[derive(Debug,Deserialize, Serialize,Default)]
pub struct CheckRequest{
    pub default : ProcessorSummary,
    pub fallback: ProcessorSummary,
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct ProcessorSummary {
    #[serde(rename = "totalRequests")]
    pub total_requests: usize,
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