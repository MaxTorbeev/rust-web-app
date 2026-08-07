use crate::{PresenceAction, ProtocolMessage};
use crate::transport::SocketContext;

pub async fn presence(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      if !context.channel_hub.is_attached(channel, &context.connection.id).await {
        ProtocolMessage::nack(message.msg_serial)
      } else {
        let incoming_presence = message.presence.clone().unwrap_or_default();
        let mut changed_presence = Vec::new();

        for presence in incoming_presence {
          let changed = match presence.action.clone() {
            PresenceAction::Enter => {
              Some(context.presence_hub.enter(channel, context.connection, presence).await)
            }
            PresenceAction::Update => {
              context.presence_hub.update(channel, context.connection, presence).await
            }
            PresenceAction::Leave => {
              context.presence_hub.leave(channel, &context.connection.id).await
            }
            _ => None
          };

          if let Some(presence) = changed {
            changed_presence.push(presence);
          }
        }

        if changed_presence.is_empty() {
          ProtocolMessage::nack(message.msg_serial)
        } else {
          context
            .channel_hub
            .broadcast(channel, ProtocolMessage::presence(channel, changed_presence))
            .await;

          ProtocolMessage::ack(&message)
        }
      }
    }
    None => ProtocolMessage::nack(message.msg_serial)
  }
}