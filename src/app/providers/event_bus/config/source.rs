use confique::Config;
use serde::Deserialize;

#[derive(Config)]
#[config(layer_attr(serde(deny_unknown_fields)))]
pub(crate) struct EventBusConfigSource {
  #[config(default = "local", env = "EVENT_BUS_DRIVER")]
  pub(crate) driver: EventBusDriverSource,

  #[config(env = "APP")]
  pub(crate) app: Option<String>,

  #[config(env = "APP_ENV")]
  pub(crate) app_environment: Option<String>,

  #[config(nested)]
  pub(crate) nats: NatsConfigSource,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EventBusDriverSource {
  Local,
  Nats,
}

#[derive(Config)]
#[config(layer_attr(serde(deny_unknown_fields)))]
pub(crate) struct NatsConfigSource {
  #[config(
    env = "NATS_SERVERS",
    parse_env = confique::env::parse::list_by_comma
  )]
  pub(crate) servers: Option<Vec<String>>,

  #[config(env = "APP_NODE_ID")]
  pub(crate) node_id: Option<String>,

  #[config(nested)]
  pub(crate) stream: NatsStreamConfigSource,

  #[config(nested)]
  pub(crate) consumer: NatsConsumerConfigSource,

  #[config(nested)]
  pub(crate) processing: EventProcessingConfigSource,
}

#[derive(Config)]
#[config(layer_attr(serde(deny_unknown_fields)))]
pub(crate) struct NatsStreamConfigSource {
  #[config(env = "NATS_STREAM_NAME")]
  pub(crate) name: Option<String>,

  #[config(default = 100_000, env = "NATS_STREAM_MAX_MESSAGES")]
  pub(crate) max_messages: i64,

  #[config(default = 1_048_576, env = "NATS_STREAM_MAX_MESSAGE_SIZE_BYTES")]
  pub(crate) max_message_size_bytes: i32,

  #[config(default = 1_073_741_824, env = "NATS_STREAM_MAX_BYTES")]
  pub(crate) max_bytes: i64,

  #[config(default = 86_400, env = "NATS_STREAM_MAX_AGE_SECONDS")]
  pub(crate) max_age_seconds: u64,

  #[config(default = 1, env = "NATS_STREAM_REPLICAS")]
  pub(crate) replicas: usize,

  #[config(default = 120, env = "NATS_STREAM_DUPLICATE_WINDOW_SECONDS")]
  pub(crate) duplicate_window_seconds: u64,
}

#[derive(Config)]
#[config(layer_attr(serde(deny_unknown_fields)))]
pub(crate) struct NatsConsumerConfigSource {
  #[config(default = 60, env = "NATS_CONSUMER_ACK_WAIT_SECONDS")]
  pub(crate) ack_wait_seconds: u64,

  #[config(default = 5, env = "NATS_CONSUMER_MAX_DELIVER")]
  pub(crate) max_deliver: i64,

  #[config(default = 5, env = "NATS_CONSUMER_RETRY_DELAY_SECONDS")]
  pub(crate) retry_delay_seconds: u64,
}

#[derive(Config)]
#[config(layer_attr(serde(deny_unknown_fields)))]
pub(crate) struct EventProcessingConfigSource {
  #[config(default = 20, env = "EVENT_BUS_PROCESSING_TIMEOUT_SECONDS")]
  pub(crate) timeout_seconds: u64,

  #[config(default = 30, env = "EVENT_BUS_LEASE_TTL_SECONDS")]
  pub(crate) lease_ttl_seconds: u64,

  #[config(default = 86_400, env = "EVENT_BUS_COMPLETED_RECORD_TTL_SECONDS")]
  pub(crate) completed_record_ttl_seconds: u64,
}
