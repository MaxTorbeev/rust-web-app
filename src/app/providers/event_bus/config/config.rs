use event_bus::IncomingEventProcessorConfig;
use event_bus_dedup_redis::RedisDedupStoreConfig;
use event_bus_jetstream::{JetStreamIncomingConsumerConfig, JetStreamSubjectConfig};
use nats_client::{ConsumerConfig, NatsConfig, StreamConfig};

pub(crate) enum EventBusConfig {
  Local,
  JetStream(JetStreamEventBusConfig),
}

pub(crate) struct JetStreamEventBusConfig {
  pub(crate) nats: NatsConfig,
  pub(crate) stream: StreamConfig,
  pub(crate) consumer: ConsumerConfig,
  pub(crate) subjects: JetStreamSubjectConfig,
  pub(crate) dedup: RedisDedupStoreConfig,
  pub(crate) processor: IncomingEventProcessorConfig,
  pub(crate) incoming: JetStreamIncomingConsumerConfig,
}
