use super::request::build_command;
use crate::transport::SocketContext;
use crate::{PresenceMutationOutcome, ProtocolMessage};

pub(crate) async fn presence(
  message: ProtocolMessage,
  context: &SocketContext<'_>,
) -> ProtocolMessage {
  let command = match build_command(&message, context.connection) {
    Ok(command) => command,
    Err(error) => {
      tracing::warn!(%error, "invalid presence request");
      return ProtocolMessage::nack(message.msg_serial);
    }
  };

  match context.presence.apply(command).await {
    Ok(PresenceMutationOutcome::Committed(_)) => ProtocolMessage::ack(&message),
    Ok(PresenceMutationOutcome::Rejected(rejection)) => {
      tracing::debug!(?rejection, "presence request rejected");
      ProtocolMessage::nack(message.msg_serial)
    }
    Err(error) => {
      tracing::error!(%error, "failed to apply presence request");
      ProtocolMessage::nack(message.msg_serial)
    }
  }
}
