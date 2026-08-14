use async_nats::jetstream::Message as DriverMessage;
use bytes::Bytes;
use crate::AckError;

pub struct NatsMessage {
  inner: DriverMessage
}

impl NatsMessage {
  pub(crate) fn from_driver(inner: DriverMessage) -> Self {
    Self { inner }
  }

  pub fn subject(&self) -> &str {
    self.inner.message.subject.as_str()
  }

  pub fn payload(&self) -> &Bytes {
    &self.inner.message.payload
  }

  pub async fn ack(&self) -> Result<(), AckError> {
    self.inner
      .ack()
      .await
      .map_err(AckError::from_driver)
  }
}