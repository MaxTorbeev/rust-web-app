use crate::transport::{SocketContext, attach};
use crate::{
  ProtocolAction, ProtocolMessage, ProtocolOutcome, auth, detach, message as message_handler,
  presence,
};

pub async fn handle_protocol_message(
  message: ProtocolMessage,
  context: &SocketContext<'_>,
) -> ProtocolOutcome {
  match message.action {
    ProtocolAction::Connect => {
      ProtocolOutcome::reply(ProtocolMessage::connected(context.connection))
    }
    ProtocolAction::Auth => ProtocolOutcome::reply(auth(message, context).await),
    ProtocolAction::Disconnect => ProtocolOutcome::disconnect(ProtocolMessage::disconnected()),
    ProtocolAction::Attach => ProtocolOutcome::replies(attach(message, context).await),
    ProtocolAction::Presence => ProtocolOutcome::reply(presence(message, context).await),
    ProtocolAction::Message => ProtocolOutcome::reply(message_handler(message, context).await),
    ProtocolAction::Heartbeat => ProtocolOutcome::reply(ProtocolMessage::heartbeat()),
    ProtocolAction::Detach => ProtocolOutcome::reply(detach(message, context).await),
    _ => ProtocolOutcome::no_reply(),
  }
}
