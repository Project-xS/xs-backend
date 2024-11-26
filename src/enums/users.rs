use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Test {
    pub message: String,
    pub hi: u32
}