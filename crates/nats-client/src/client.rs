use crate::{
  ConnectError, ConsumerConfig, NatsConfig, NatsSubscription, PublishAck, PublishError,
  PublishMessage, StreamConfig, StreamSetupError, SubscribeError, TopologyError,
};
/// Generic access to the NATS JetStream transport.
///
/// Основная точка доступа к NATS JetStream: подключение, публикация, подготовка
/// потоков и создание подписок. Структура скрывает типы драйвера `async-nats`.
pub struct NatsClient {
  jetstream: async_nats::jetstream::Context,
}

impl NatsClient {
  /// Connects to the configured NATS server or cluster.
  ///
  /// Подключается к указанному серверу или кластеру NATS и создаёт контекст
  /// JetStream.
  pub async fn connect(config: &NatsConfig) -> Result<Self, ConnectError> {
    let connection = async_nats::connect(config.servers())
      .await
      .map_err(ConnectError::from_driver)?;

    let jetstream = async_nats::jetstream::new(connection);

    Ok(Self { jetstream })
  }

  /// Publishes and waits until JetStream confirms stream acceptance.
  ///
  /// Публикует подготовленное сообщение и ждёт подтверждения, что JetStream
  /// принял его в подходящий поток. Это не подтверждение обработки consumer-ом.
  pub async fn publish(&self, message: PublishMessage) -> Result<PublishAck, PublishError> {
    let (subject, message) = message.into_driver();

    let ack = self
      .jetstream
      .send_publish(subject, message)
      .await
      .map_err(PublishError::from_driver)?;

    let ack = ack.await.map_err(PublishError::from_driver)?;

    Ok(PublishAck::from_driver(ack))
  }

  /// Creates a missing stream or verifies that an existing stream is compatible.
  ///
  /// Existing streams are never updated implicitly because reducing retention
  /// limits during application startup could discard persisted messages.
  ///
  /// Создаёт отсутствующий поток или проверяет совместимость существующего.
  /// Существующий поток не изменяется автоматически, потому что уменьшение
  /// лимитов хранения может удалить уже сохранённые сообщения.
  pub async fn ensure_stream(&self, config: &StreamConfig) -> Result<(), StreamSetupError> {
    let stream = self
      .jetstream
      .get_or_create_stream(config.to_driver_config())
      .await
      .map_err(StreamSetupError::from_driver)?;

    let incompatible_fields = config.incompatible_fields(&stream.cached_info().config);
    if !incompatible_fields.is_empty() {
      return Err(StreamSetupError::incompatible_configuration(
        config.name(),
        incompatible_fields,
      ));
    }

    Ok(())
  }

  /// Verifies the configured JetStream stream and durable consumer without
  /// creating or updating either resource.
  ///
  /// Выполняет свежие `STREAM.INFO` и `CONSUMER.INFO`, после чего проверяет
  /// совместимость фактической конфигурации с ожидаемой. Метод ничего не
  /// создаёт и не исправляет.
  pub async fn verify_topology(
    &self,
    stream_config: &StreamConfig,
    consumer_config: &ConsumerConfig,
  ) -> Result<(), TopologyError> {
    if consumer_config.stream_name() != stream_config.name() {
      return Err(TopologyError::consumer_stream_mismatch(
        consumer_config.durable_name(),
        consumer_config.stream_name(),
        stream_config.name(),
      ));
    }

    let connection_state = self.jetstream.client().connection_state();
    if connection_state != async_nats::connection::State::Connected {
      return Err(TopologyError::core_unavailable(connection_state));
    }

    let stream = self
      .jetstream
      .get_stream(stream_config.name())
      .await
      .map_err(|source| TopologyError::stream_info(stream_config.name(), source))?;

    let incompatible_fields = stream_config.incompatible_fields(&stream.cached_info().config);
    if !incompatible_fields.is_empty() {
      return Err(TopologyError::stream_configuration(
        stream_config.name(),
        incompatible_fields,
      ));
    }

    let consumer = stream
      .consumer_info(consumer_config.durable_name())
      .await
      .map_err(|source| TopologyError::consumer_info(consumer_config.durable_name(), source))?;

    let incompatible_fields = consumer_config.incompatible_fields(&consumer.config);
    if !incompatible_fields.is_empty() {
      return Err(TopologyError::consumer_configuration(
        consumer_config.durable_name(),
        incompatible_fields,
      ));
    }

    Ok(())
  }

  /// Opens the delivery stream for a compatible durable pull consumer.
  ///
  /// Создаёт отсутствующего durable pull consumer-а или проверяет существующего,
  /// после чего открывает поток входящих доставок.
  pub async fn subscribe(
    &self,
    config: &ConsumerConfig,
  ) -> Result<NatsSubscription, SubscribeError> {
    let stream = self
      .jetstream
      .get_stream(config.stream_name())
      .await
      .map_err(SubscribeError::stream)?;

    let consumer = stream
      .get_or_create_consumer(config.durable_name(), config.to_driver_config())
      .await
      .map_err(SubscribeError::consumer)?;

    let incompatible_fields = config.incompatible_fields(&consumer.cached_info().config);
    if !incompatible_fields.is_empty() {
      return Err(SubscribeError::incompatible_configuration(
        config.durable_name(),
        incompatible_fields,
      ));
    }

    let messages = consumer
      .messages()
      .await
      .map_err(SubscribeError::messages)?;

    Ok(NatsSubscription::new(messages))
  }
}
