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

  let receipt = match context.presence.apply(command).await {
    Ok(receipt) => receipt,
    Err(error) => {
      tracing::error!(%error, "failed to apply presence request");
      return ProtocolMessage::nack(message.msg_serial);
    }
  };

  // Повтор команды клиентом — штатная ситуация (потерянный ACK, resume), но
  // его частота — сигнал о качестве соединения, поэтому он виден в трассировке.
  match receipt.outcome {
    PresenceMutationOutcome::Committed(_) => {
      tracing::debug!(
        connection_id = context.connection.id.as_str(),
        msg_serial = ?message.msg_serial,
        replayed = receipt.replayed,
        "presence request committed"
      );

      ProtocolMessage::ack(&message)
    }
    PresenceMutationOutcome::Rejected(rejection) => {
      tracing::debug!(
        connection_id = context.connection.id.as_str(),
        msg_serial = ?message.msg_serial,
        replayed = receipt.replayed,
        ?rejection,
        "presence request rejected"
      );

      ProtocolMessage::nack(message.msg_serial)
    }
  }
}
