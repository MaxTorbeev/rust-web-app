use crate::protocol_handlers::SocketContext;
use crate::{ProtocolFlag, ProtocolMessage};

pub async fn attach(message: ProtocolMessage, context: &SocketContext<'_>) -> Vec<ProtocolMessage> {
  let Some(channel) = message.channel.as_deref() else {
    return vec![ProtocolMessage::nack(message.msg_serial)]
  };

  context
    .channel_hub
    .attach(
      channel,
      context.connection.id.clone(),
      context.sender.clone()
    )
    .await;

  let presence = context.presence_hub.snapshot(channel).await;

  vec![
    // Отправить оповещение о том что клиент добавлен
    ProtocolMessage::attached(&message, ProtocolFlag::HAS_PRESENCE),
    // Отправить snapshot присутствующих клиентов
    ProtocolMessage::sync(channel, presence)
  ]
}