use uuid::Uuid;
use support::timestamp::Timestamp;
use crate::connection::ConnectionActor;

pub struct DisconnectConnectionCommand {
  pub actor: ConnectionActor,
  pub operation_id: Uuid,
  pub normalized_request_hash: String,
  pub request_time: Timestamp,
}