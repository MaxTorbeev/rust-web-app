use std::time::Duration;

use super::RedisConfig;

#[test]
fn default_is_a_deterministic_local_configuration() {
  let config = RedisConfig::default();

  assert_eq!(config.host, "127.0.0.1");
  assert_eq!(config.port, "6379");
  assert_eq!(config.username, None);
  assert_eq!(config.password, None);
  assert_eq!(config.db, "0");
  assert_eq!(config.connection_timeout, Duration::from_secs(5));
  assert_eq!(config.response_timeout, Duration::from_secs(3));
}
