use crate::{
  ConnectError, ConsumerConfig, NatsConfig, NatsSubscription, PublishAck,
  PublishError, PublishMessage, StreamConfig, StreamSetupError, SubscribeError,
};

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

  pub async fn publish(
    &self,
    message: PublishMessage,
  ) -> Result<PublishAck, PublishError> {
    let (subject, message) = message.into_driver();

    // The first await sends the message and creates an ACK future.
    let ack = self
      .jetstream
      .send_publish(subject, message)
      .await
      .map_err(PublishError::from_driver)?;

    // The second await confirms that JetStream accepted it into a stream.
    let ack = ack
      .await
      .map_err(PublishError::from_driver)?;

    Ok(PublishAck::from_driver(ack))
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

  pub async fn subscribe(&self, config: &ConsumerConfig) -> Result<NatsSubscription, SubscribeError> {
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

    Ok(NatsSubscription::new(messages))
  }
}
