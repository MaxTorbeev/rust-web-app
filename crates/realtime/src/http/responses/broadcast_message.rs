use serde::Serialize;

#[derive(Serialize)]
pub struct BroadcastMessageResponse {
  pub sent: usize
}