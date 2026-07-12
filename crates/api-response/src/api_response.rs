use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> where T: Serialize {
  pub data: T
}

impl<T: Serialize> ApiResponse<T> where T: Serialize {
  pub fn new(data: T) -> Self {
    Self { data }
  }
}