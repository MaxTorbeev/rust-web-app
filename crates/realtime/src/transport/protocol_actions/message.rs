use crate::ProtocolMessage;
use crate::transport::SocketContext;

pub async fn message(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      context
        .channel_hub
        .broadcast(channel, message.clone())
        .await;

      ProtocolMessage::ack(&message)
    },
    None => ProtocolMessage::nack(message.msg_serial)
  }
}

