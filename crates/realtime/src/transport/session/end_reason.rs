use crate::OutboundSendError;
use crate::transport::protocol_reader::{ReaderEndReason, ReaderError, ReaderResult};
use crate::transport::SessionError;

pub(crate) enum EndReason {
  ReaderEnded(ReaderEndReason),
  ReaderFailed(ReaderError),
  ShutdownRequested,
  ProtocolFailed(OutboundSendError),
  WriterStopped(Result<(), SessionError>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriterPolicy {
  DrainUntilShutdown,
  Abort,
  AlreadyStopped
}

impl From<ReaderResult> for EndReason {
  fn from(result: ReaderResult) -> Self {
    match result {
      Ok(reason) => Self::ReaderEnded(reason),
      Err(error) => Self::ReaderFailed(error),
    }
  }
}

impl EndReason {
  pub(crate) fn writer_policy(&self) -> WriterPolicy {
    match self {
      Self::ReaderEnded(ReaderEndReason::DisconnectRequested) => {
        WriterPolicy::DrainUntilShutdown
      }

      Self::ReaderEnded(ReaderEndReason::SocketClosed | ReaderEndReason::StreamEnded)
      | Self::ReaderFailed(_)
      | Self::ShutdownRequested
      | Self::ProtocolFailed(_) => {
        WriterPolicy::Abort
      }

      Self::WriterStopped(_) => WriterPolicy::AlreadyStopped,
    }
  }

  /// Преобразовать причину остановки в Result
  pub(crate) fn into_result(self) -> Result<(), SessionError> {
    match self {
      Self::ReaderEnded(_) |
      Self::ShutdownRequested => Ok(()),

      Self::ReaderFailed(error) => {
        Err(error.into())
      }

      Self::ProtocolFailed(error) => {
        Err(error.into())
      }

      // Writer самостоятельно завершился раньше reader.
      Self::WriterStopped(Ok(())) => {
        Err(SessionError::WriterStopped)
      }

      // Writer завершился с ошибкой раньше reader.
      // Возвращаем исходную ошибку без изменений.
      Self::WriterStopped(Err(error)) => {
        Err(error)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn end_reason_selects_the_expected_writer_policy() {
    let cases = [
      (
        EndReason::from(Ok(ReaderEndReason::DisconnectRequested)),
        WriterPolicy::DrainUntilShutdown,
      ),
      (
        EndReason::from(Ok(ReaderEndReason::SocketClosed)),
        WriterPolicy::Abort,
      ),
      (
        EndReason::from(Ok(ReaderEndReason::StreamEnded)),
        WriterPolicy::Abort,
      ),
      (
        EndReason::from(Err(ReaderError::Outbound(OutboundSendError::QueueFull))),
        WriterPolicy::Abort,
      ),
      (
        EndReason::ShutdownRequested,
        WriterPolicy::Abort,
      ),
      (
        EndReason::ProtocolFailed(OutboundSendError::QueueClosed),
        WriterPolicy::Abort,
      ),
      (
        EndReason::WriterStopped(Ok(())),
        WriterPolicy::AlreadyStopped,
      ),
    ];

    for (reason, expected_policy) in cases {
      assert_eq!(reason.writer_policy(), expected_policy);
    }
  }

  #[test]
  fn normal_end_reasons_return_ok() {
    assert!(
      EndReason::ReaderEnded(ReaderEndReason::DisconnectRequested)
        .into_result()
        .is_ok()
    );
    assert!(
      EndReason::ReaderEnded(ReaderEndReason::SocketClosed)
        .into_result()
        .is_ok()
    );
    assert!(
      EndReason::ReaderEnded(ReaderEndReason::StreamEnded)
        .into_result()
        .is_ok()
    );
    assert!(EndReason::ShutdownRequested.into_result().is_ok());
  }

  #[test]
  fn failed_end_reasons_preserve_the_error_kind() {
    assert!(matches!(
      EndReason::ReaderFailed(
        ReaderError::Outbound(OutboundSendError::QueueFull)
      ).into_result(),
      Err(SessionError::Outbound(OutboundSendError::QueueFull)),
    ));

    let read_error = axum::Error::new(std::io::Error::new(
      std::io::ErrorKind::Other,
      "websocket read failed",
    ));

    assert!(matches!(
      EndReason::ReaderFailed(ReaderError::Read(read_error)).into_result(),
      Err(SessionError::Read(_)),
    ));
    assert!(matches!(
      EndReason::ProtocolFailed(OutboundSendError::QueueClosed).into_result(),
      Err(SessionError::Outbound(OutboundSendError::QueueClosed)),
    ));
    assert!(matches!(
      EndReason::WriterStopped(Ok(())).into_result(),
      Err(SessionError::WriterStopped),
    ));
  }

  #[tokio::test(flavor = "current_thread")]
  async fn writer_join_error_is_preserved() {
    // Cancellation produces a real JoinError without panicking the test.
    let task = tokio::spawn(std::future::pending::<()>());
    task.abort();

    let join_error = task
      .await
      .expect_err("cancelled writer task must return JoinError");

    assert!(matches!(
      EndReason::WriterStopped(Err(SessionError::WriterTaskFailed(join_error))).into_result(),
      Err(SessionError::WriterTaskFailed(_)),
    ));
  }
}
