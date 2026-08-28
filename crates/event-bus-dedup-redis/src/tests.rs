use event_bus::{EVENT_BUS_NAMESPACE_VERSION, EVENT_BUS_SUBSYSTEM};
use support::app::AppNamespace;

use crate::RedisDedupStoreConfig;

#[test]
fn builds_key_prefix_from_shared_namespace() {
  let namespace = AppNamespace::try_new(
    "mxt_realtime",
    "production",
    EVENT_BUS_SUBSYSTEM,
    EVENT_BUS_NAMESPACE_VERSION,
  )
  .expect("application namespace must be valid");

  let config = RedisDedupStoreConfig::new(&namespace);

  assert_eq!(
    config.key_prefix(),
    "mxt_realtime.production.event-bus.v1.dedup"
  );
}
