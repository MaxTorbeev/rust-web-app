use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use support::timestamp::Timestamp;

use super::error::PresenceRequestError;
use crate::{
  ChannelKey, Connection, PresenceActor, PresenceBatchCommand, PresenceBatchItem,
  PresenceClientIdPolicy, PresenceMutationAction, ProtocolMessage,
};

pub(super) fn build_command(
  message: &ProtocolMessage,
  connection: &Connection,
) -> Result<PresenceBatchCommand, PresenceRequestError> {
  let request_time = Timestamp::now();

  let channel = message
    .channel
    .as_deref()
    .ok_or(PresenceRequestError::MissingChannel)?;

  let msg_serial = message
    .msg_serial
    .ok_or(PresenceRequestError::MissingMessageSerial)?;

  let incoming_items = message
    .presence
    .as_deref()
    .filter(|items| !items.is_empty())
    .ok_or(PresenceRequestError::EmptyBatch)?;

  let items = incoming_items
    .iter()
    .map(|item| {
      let action = PresenceMutationAction::try_from(item.action)
        .map_err(PresenceRequestError::UnsupportedAction)?;

      let client_id = item
        .client_id
        .clone()
        .or_else(|| connection.client_id().map(str::to_owned));

      Ok(PresenceBatchItem {
        action,
        client_id,
        data: item.data.clone(),
      })
    })
    .collect::<Result<Vec<_>, PresenceRequestError>>()?;

  let request_fingerprint = compute_request_fingerprint(channel, &items)?;

  let client_id_policy = match connection.client_id() {
    Some(client_id) => PresenceClientIdPolicy::Bound(BTreeSet::from([client_id.to_owned()])),
    None => PresenceClientIdPolicy::Unidentified,
  };

  Ok(PresenceBatchCommand {
    channel: ChannelKey::new(connection.application_id().clone(), channel),
    actor: PresenceActor {
      connection_actor: connection.actor(),
      client_id_policy,
    },
    items,
    msg_serial,
    request_fingerprint,
    request_time,
  })
}

#[derive(Serialize)]
struct NormalizedPresenceBatch<'a> {
  channel: &'a str,
  items: Vec<NormalizedPresenceItem<'a>>,
}

#[derive(Serialize)]
struct NormalizedPresenceItem<'a> {
  action: &'static str,
  client_id: Option<&'a str>,
  data: &'a Option<Value>,
}

fn compute_request_fingerprint(channel: &str, items: &[PresenceBatchItem]) -> Result<String, serde_json::Error> {
  let items = items
    .iter()
    .map(|item| NormalizedPresenceItem {
      action: item.action.as_str(),
      client_id: item.client_id.as_deref(),
      data: &item.data,
    })
    .collect();

  let normalized = NormalizedPresenceBatch { channel, items };
  let payload = serde_json::to_vec(&normalized)?;

  Ok(hex::encode(Sha256::digest(payload)))
}