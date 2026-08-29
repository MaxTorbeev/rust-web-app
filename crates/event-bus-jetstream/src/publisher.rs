use std::sync::Arc;

use bytes::Bytes;
use event_bus::{DeliveryClass, EventBusError, EventMessage, EventPublishFuture, EventPublisher};
use nats_client::{NatsClient, PublishMessage};

use crate::subject::event_subject;
use crate::subject_config::JetStreamSubjectConfig;

/// Publishes distributed event envelopes to NATS JetStream.
///
/// A successful publication means that JetStream accepted the envelope into a
/// matching stream. It does not mean that a consumer processed the event.
pub struct JetStreamEventPublisher {
  client: Arc<NatsClient>,
  subjects: JetStreamSubjectConfig,
}

impl JetStreamEventPublisher {
  /// Creates a publisher from an already connected client and subject topology.
  pub fn new(client: Arc<NatsClient>, subjects: JetStreamSubjectConfig) -> Self {
    Self { client, subjects }
  }
}

impl EventPublisher for JetStreamEventPublisher {
  fn publish<'a>(
    &'a self,
    message: &'a EventMessage,
    delivery: DeliveryClass,
  ) -> EventPublishFuture<'a> {
    Box::pin(async move {
      let outgoing = prepare_message(&self.subjects, message, delivery)?;
      let ack = self
        .client
        .publish(outgoing)
        .await
        .map_err(EventBusError::publisher)?;

      tracing::debug!(
          event_id = %message.event_id(),
          event_name = message.event_name(),
          delivery = ?delivery,
          stream = %ack.stream,
          sequence = ack.sequence,
          duplicate = ack.duplicate,
          "event accepted by JetStream",
      );

      Ok(())
    })
  }
}

pub(crate) fn prepare_message(
  subjects: &JetStreamSubjectConfig,
  message: &EventMessage,
  delivery: DeliveryClass,
) -> Result<PublishMessage, EventBusError> {
  let subject = event_subject(subjects.prefix(), message.event_name(), delivery)
    .map_err(EventBusError::publisher)?;

  let payload = Bytes::from(message.to_bytes()?);

  PublishMessage::new(subject, payload)
    .map_err(EventBusError::publisher)?
    .message_id(message.event_id().to_string())
    .map_err(EventBusError::publisher)
}
