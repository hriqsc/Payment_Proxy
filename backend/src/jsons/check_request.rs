use serde::{Deserialize, Serialize};

#[derive(Debug,Deserialize, Serialize,Default)]
pub struct CheckRequest{
    pub default : isize,
    pub fallback: isize,
}