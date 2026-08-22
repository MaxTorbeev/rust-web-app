use std::sync::Arc;
use bytes::Bytes;
use event_bus::{DeliveryClass, EventBusError, EventMessage, EventPublishFuture, EventPublisher};
use nats_client::{NatsClient, PublishMessage};
use crate::config::JetStreamPublisherConfig;
use crate::JetStreamPublisherError;
use crate::event_subject;

pub struct JetStreamEventPublisher {
  client: Arc<NatsClient>,
  config: JetStreamPublisherConfig,
}

impl JetStreamEventPublisher {
  pub fn new(client: Arc<NatsClient>, config: JetStreamPublisherConfig) -> Self {
    Self {
      client,
      config,
    }
  }

  pub fn try_new(client: Arc<NatsClient>) -> Result<Self, JetStreamPublisherError> {
    Ok(Self {
      client,
      config: JetStreamPublisherConfig::try_from_env()?,
    })
  }
}

impl EventPublisher for JetStreamEventPublisher {
  fn publish<'a>(&'a self, message: &'a EventMessage, delivery: DeliveryClass) -> EventPublishFuture<'a> {

    Box::pin(async move {

      // Получить адрес в NATS
      let subject = event_subject(&self.config.subject_prefix, message.event_name(), delivery)
        .map_err(EventBusError::publisher)?;

      // Преобразовать сообщение в байты
      let payload = Bytes::from(message.to_bytes()?);

      let event_id = message.event_id();

      let outgoing = PublishMessage::new(subject, payload)
        .and_then(|outgoing| {
          outgoing.message_id(event_id.to_string())
        })
        .map_err(EventBusError::publisher)?;

      let ack = self
        .client
        .publish(outgoing)
        .await
        .map_err(EventBusError::publisher)?;

      tracing::debug!(
        event_id = %message.event_id(),
        event_name = message.event_name(),
        stream = %ack.stream,
        sequence = ack.sequence,
        duplicate = ack.duplicate,
        "event accepted by JetStream",
      );

      Ok(())
    })
  }
}
