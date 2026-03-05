use serde::{Deserialize, Serialize};

#[derive(Debug,Deserialize, Serialize)]
pub struct CheckRequest{
    pub default : i32,
    pub fallback: i32,
}