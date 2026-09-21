use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use redis_client::ScriptValue;
use redis_lease::{LeaseToken, lease_value};
use serde::Deserialize;
use support::{NodeInstance, fresh_uuid, timestamp::Timestamp};

use super::{
  RedisChannelStore,
  protocol::generation,
  response::{decode_rejection, decode_text},
  scripts,
};
use crate::{
  ApplicationId, ChannelStateStoreError, ConnectionActor, ConnectionId, DisconnectConnectionCommand,
};

impl RedisChannelStore {
  /// Очищает ограниченную порцию соединений. true означает финализацию registry.
  pub(super) async fn reap_generation_batch(
    &self,
    target: &NodeInstance,
    token: &LeaseToken,
    limit: u32,
  ) -> Result<bool, ChannelStateStoreError> {
    if token.key().as_str() != self.keys.cleanup_lease(target)
      || token.owner().as_str() != generation(self.node_lease.instance())
      || limit == 0
    {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "invalid generation cleanup token or batch size".into(),
      });
    }
    let keys = [
      self.keys.node_lease(&self.node_lease.instance().node_id),
      self.keys.generation_deadlines(),
      token.key().as_str().to_owned(),
      self.keys.node_lease(&target.node_id),
      self.keys.generation_connections(target),
      self.keys.generation_shards(target),
      self.keys.generations(),
    ];
    let args = [
      lease_value(self.node_lease.token()),
      generation(self.node_lease.instance()),
      lease_value(token),
      generation(target),
      limit.to_string(),
    ];
    let reply = self
      .redis
      .invoke_script(
        scripts::REAP_GENERATION,
        &keys.each_ref().map(|v| v.as_bytes()),
        &args.each_ref().map(|v| v.as_bytes()),
      )
      .await
      .map_err(|e| ChannelStateStoreError::Internal {
        message: e.to_string(),
      })?;
    let invalid = || ChannelStateStoreError::Internal {
      message: "invalid generation cleanup reply".into(),
    };
    let ScriptValue::Array(reply) = reply else {
      return Err(invalid());
    };
    let (connections, done) = match reply.as_slice() {
      [
        ScriptValue::Integer(1),
        ScriptValue::Array(connections),
        ScriptValue::Integer(done),
      ] => (connections, *done == 1),
      [ScriptValue::Integer(0), code, message] => {
        return Err(decode_rejection(
          decode_text(code, "code")?,
          decode_text(message, "message")?.into(),
        ));
      }
      _ => return Err(invalid()),
    };
    for reference in connections {
      let reference = decode_text(reference, "connection reference")?;
      let (app, connection) = reference.split_once('.').ok_or_else(invalid)?;
      let decode = |value: &str| {
        String::from_utf8(URL_SAFE_NO_PAD.decode(value).map_err(|_| invalid())?)
          .map_err(|_| invalid())
      };
      let connection = decode(connection)?;
      let connection_id = ConnectionId::deserialize(serde::de::value::StrDeserializer::<
        serde::de::value::Error,
      >::new(&connection))
      .map_err(|_| invalid())?;
      self
        .reap_connection(
          DisconnectConnectionCommand {
            actor: ConnectionActor {
              application_id: ApplicationId::new(decode(app)?),
              connection_id,
              node_instance: target.clone(),
            },
            request_time: Timestamp::now(),
            event_id: fresh_uuid(),
          },
          token,
        )
        .await?;
    }
    Ok(done)
  }
}
