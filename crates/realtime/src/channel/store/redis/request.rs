use serde::Serialize;

use crate::PresenceBatchCommand;

use super::member::MemberPayload;
use super::protocol::segment;

/// Готовит данные batch без чтения Redis; отказы по client_id применяет Lua
/// в порядке элементов, после поиска повтора и проверки attachment.
pub(super) fn encode_presence_items(command: &PresenceBatchCommand) -> serde_json::Result<String> {
  #[derive(Serialize)]
  #[serde(rename_all = "camelCase")]
  struct Item<'a> {
    action: &'static str,
    client_id: Option<&'a str>,
    client_segment: Option<String>,
    allowed: bool,
    has_data: bool,
    member_json: Option<String>,
  }

  let items = command
    .items
    .iter()
    .enumerate()
    .map(|(index, item)| {
      let client_id = item.client_id.as_deref();
      let member_json = client_id
        .map(|client_id| {
          serde_json::to_string(&MemberPayload {
            connection_id: command.actor.connection_actor.connection_id.clone(),
            client_id: client_id.to_owned(),
            node_instance: command.actor.connection_actor.node_instance.clone(),
            data_json: item.data.as_ref().map(serde_json::to_string).transpose()?,
            last_message_id: command.message_id(index),
          })
        })
        .transpose()?;
      Ok(Item {
        action: item.action.as_str(),
        client_id,
        client_segment: client_id.map(segment),
        allowed: client_id.is_some_and(|id| command.actor.client_id_policy.allows(id)),
        has_data: item.data.is_some(),
        member_json,
      })
    })
    .collect::<serde_json::Result<Vec<_>>>()?;
  serde_json::to_string(&items)
}

/// Готовит JSON каналов и стабильные event ID после чтения connection index.
pub(super) fn encode_disconnect_channels(
  command: &crate::DisconnectConnectionCommand,
  channels: &[crate::ChannelKey],
) -> serde_json::Result<String> {
  #[derive(Serialize)]
  #[serde(rename_all = "camelCase")]
  struct Channel {
    segment: String,
    channel_json: String,
    event_id: uuid::Uuid,
  }
  let channels = channels
    .iter()
    .map(|channel| {
      Ok(Channel {
        segment: segment(&channel.channel),
        channel_json: serde_json::to_string(channel)?,
        event_id: command.channel_event_id(channel),
      })
    })
    .collect::<serde_json::Result<Vec<_>>>()?;
  serde_json::to_string(&channels)
}
