use bytes::Bytes;
use crate::{ConnectError, ConsumerConfig, NatsConfig, PublishError, StreamConfig, StreamSetupError, SubscribeError, Subscription};

pub struct NatsClient {
  /// Точка доступа ко всему JetStream api
  jetstream: async_nats::jetstream::Context,
}

impl NatsClient {
  pub async fn connect(config: &NatsConfig) -> Result<Self, ConnectError> {
    let connection = async_nats::connect(&config.servers)
      .await
      .map_err(ConnectError::from_driver)?;

    let jetstream = async_nats::jetstream::new(connection);

    Ok(Self { jetstream })
  }

  pub async fn publish(&self, subject: impl Into<String>, payload: Bytes) -> Result<(), PublishError> {
    // Отправили сообщение и получили future подтверждения
    let ack = self.jetstream
      .publish(subject.into(), payload)
      .await
      .map_err(PublishError::from_driver)?;

    // JetStream подтвердил, что сообщение принято и добавлено в stream
    ack
      .await
      .map_err(PublishError::from_driver)?;

    Ok(())
  }

  pub async fn get_or_create_stream(&self, config: StreamConfig) ->Result<(), StreamSetupError> {
    let config = config.into_driver_config();

    self
      .jetstream
      .get_or_create_stream(config)
      .await
      .map_err(StreamSetupError::from_driver)?;

    Ok(())
  }

  pub async fn subscribe(&self, config: &ConsumerConfig) -> Result<Subscription, SubscribeError> {
    let stream = self.jetstream
      .get_stream(&config.stream_name)
      .await
      .map_err(SubscribeError::stream)?;

    let consumer = stream
      .get_or_create_consumer(
        &config.durable_name,
        config.to_driver_config(),
      )
      .await
      .map_err(SubscribeError::consumer)?;

    let messages = consumer
      .messages()
      .await
      .map_err(SubscribeError::messages)?;

    Ok(Subscription::new(messages))
  }
}