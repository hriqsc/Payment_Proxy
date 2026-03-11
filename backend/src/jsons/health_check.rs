
use serde::{Deserialize, Serialize};

/// Response structure returned by a processor health check endpoint.
///
/// Indicates whether the processor is currently failing and
/// provides the minimum response time observed.
#[derive(Serialize, Deserialize)]
pub struct HealthCheck {

    /// Indicates if the processor is currently failing.
    /// When `true`, the processor should be considered unavailable.
    pub failing : bool,

    /// Minimum response time (in milliseconds) reported by the processor.
    #[serde(rename = "minResponseTime")]
    pub min_response_time : u64
}

/*
https://github.com/zanfranceschi/rinha-de-backend-2025/blob/main/INSTRUCOES.md#payments

GET /payments/service-health

HTTP 200 - Ok
{
    "failing": false,
    "minResponseTime": 100
}

*/