use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastMessageResponse {
  pub accepted: bool,
  pub event_id: Uuid
}