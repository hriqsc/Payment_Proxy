use serde::{Deserialize, Serialize};


#[derive(Clone, Debug,Deserialize, Serialize,Default)]
pub struct PaymentRequest {
    pub amount: f64,
    #[serde(rename = "correlationId")]
    pub id : String,
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