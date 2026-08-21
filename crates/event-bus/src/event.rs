use serde::Serialize;
use serde::de::DeserializeOwned;

/// Defines where an event must be delivered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryClass {
    /// Apply the event only in the publishing process.
    LocalOnly,

    /// Apply the event independently on every application node.
    AllNodes,

    /// Apply the event once within a shared consumer group.
    WorkQueue,
}

/// A serializable domain event that can be published through [`crate::EventBus`].
///
/// Event values are ordinary Rust structs. Implement this trait to declare the
/// stable wire name, schema version and delivery policy, then register one
/// required handler before sharing the dispatcher.
///
/// ```no_run
/// use std::sync::Arc;
///
/// use event_bus::{
///     DeliveryClass, Event, EventBus, EventBusError, EventDispatcher,
/// };
/// use serde::{Deserialize, Serialize};
///
/// // 1. Declare the event payload.
/// #[derive(Debug, Deserialize, Serialize)]
/// struct UserCreated {
///     user_id: u64,
/// }
///
/// // 2. Declare its wire contract.
/// impl Event for UserCreated {
///     const NAME: &'static str = "users.user_created";
///     const VERSION: u16 = 1;
///     const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), EventBusError> {
/// // 3. Register the required handler during application startup.
/// let mut dispatcher = EventDispatcher::new();
/// dispatcher.register(|event: UserCreated| async move {
///     println!("created user {}", event.user_id);
///     Ok(())
/// })?;
///
/// // 4. Build the bus and publish event values from application code.
/// let event_bus = EventBus::local(Arc::new(dispatcher));
/// event_bus.publish(UserCreated { user_id: 42 }).await?;
///
/// # Ok(())
/// # }
/// ```
pub trait Event: Send + Sync + Serialize + DeserializeOwned + 'static {
    /// Stable and unique wire name. Do not rename it after events are persisted.
    const NAME: &'static str;

    /// Payload schema version. Increase it for incompatible payload changes.
    const VERSION: u16 = 1;

    /// Delivery policy. Events are local unless explicitly configured otherwise.
    const DELIVERY: DeliveryClass = DeliveryClass::LocalOnly;
}
