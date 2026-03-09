
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HealthCheck{
    pub failing : bool,
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