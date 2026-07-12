use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
  message: String,
}

impl ApiMessage {
  pub fn new(message: String) -> Self {
    Self { message }
  }
}