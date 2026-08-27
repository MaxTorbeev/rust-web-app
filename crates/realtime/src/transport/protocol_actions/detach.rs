use crate::ProtocolMessage;
use crate::transport::SocketContext;

pub async fn detach(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      let leave = context
        .presence_hub
        .leave(channel, &context.connection.id)
        .await;

      context
        .channel_hub
        .detach(channel, &context.connection.id)
        .await;

      if let Some(presence) = leave {
        if let Err(error) = context
          .channel_hub
          .broadcast(channel, ProtocolMessage::presence(channel, vec![presence]))
          .await
        {
          tracing::error!(%error, %channel, "failed to broadcast presence leave");
        }
      }

      ProtocolMessage::detached(&message)
    }
    None => ProtocolMessage::nack(message.msg_serial),
  }
}
