use event_bus::{Event, EventBus, EventMessage};
use redis_client::ScriptValue;
use redis_lease::{LeaseToken, lease_value};

use super::{
  RedisChannelStore,
  protocol::generation,
  response::{decode_outbox, decode_rejection, decode_text},
  scripts,
};
use crate::{ChannelStateStoreError, PresenceChannelChanged};

impl RedisChannelStore {
  async fn outbox(
    &self,
    token: &LeaseToken,
    operation: &str,
    value: &str,
  ) -> Result<Vec<ScriptValue>, ChannelStateStoreError> {
    if token.key().as_str() != self.keys.publisher_lease()
      || token.owner().as_str() != generation(self.node_lease.instance())
    {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "invalid outbox publisher token".into(),
      });
    }
    let keys = [
      self.keys.node_lease(&self.node_lease.instance().node_id),
      self.keys.generation_deadlines(),
      self.keys.publisher_lease(),
      self.keys.outbox(),
    ];
    let args = [
      lease_value(self.node_lease.token()),
      generation(self.node_lease.instance()),
      lease_value(token),
      operation.into(),
      value.into(),
    ];
    let reply = self
      .redis
      .invoke_script(
        scripts::OUTBOX,
        &keys.each_ref().map(|v| v.as_bytes()),
        &args.each_ref().map(|v| v.as_bytes()),
      )
      .await
      .map_err(|e| ChannelStateStoreError::Internal {
        message: e.to_string(),
      })?;
    if let ScriptValue::Array(mut fields) = reply {
      match fields.as_slice() {
        [ScriptValue::Integer(1), ScriptValue::Array(_)] => {
          let ScriptValue::Array(entries) = fields.pop().unwrap() else {
            unreachable!()
          };
          return Ok(entries);
        }
        [ScriptValue::Integer(0), code, message] => {
          return Err(decode_rejection(
            decode_text(code, "code")?,
            decode_text(message, "message")?.into(),
          ));
        }
        _ => {}
      }
    }
    Err(ChannelStateStoreError::Internal {
      message: "invalid outbox reply".into(),
    })
  }

  /// ACK удаляет запись только после успешной публикации и повторной проверки fencing.
  pub(super) async fn publish_outbox_batch(
    &self,
    token: &LeaseToken,
    bus: &EventBus,
  ) -> Result<usize, ChannelStateStoreError> {
    let entries = self.outbox(token, "read", "32").await?;
    let count = entries.len();
    for entry in entries {
      let invalid = || ChannelStateStoreError::Internal {
        message: "invalid outbox entry".into(),
      };
      let ScriptValue::Array(fields) = entry else {
        return Err(invalid());
      };
      let [id, payload] = fields.as_slice() else {
        return Err(invalid());
      };
      let id = decode_text(id, "stream ID")?;
      let event = decode_outbox(payload)?;
      let message =
        EventMessage::try_from(&event).map_err(|e| ChannelStateStoreError::Internal {
          message: e.to_string(),
        })?;
      bus
        .publish_message(&message, PresenceChannelChanged::DELIVERY)
        .await
        .map_err(|e| ChannelStateStoreError::Internal {
          message: e.to_string(),
        })?;
      self.outbox(token, "ack", id).await?;
    }
    Ok(count)
  }
}
