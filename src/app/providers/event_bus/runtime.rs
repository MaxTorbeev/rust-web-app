use super::{EventBusHealthCheck, EventBusRuntimeError};
use event_bus::EventBus;
use event_bus_jetstream::JetStreamIncomingConsumer;
use std::future::pending;
use std::sync::Arc;

pub struct EventBusRuntime {
  event_bus: Arc<EventBus>,
  worker: Option<JetStreamIncomingConsumer>,
  health: EventBusHealthCheck,
}

impl EventBusRuntime {
  pub(super) fn local(event_bus: Arc<EventBus>) -> Self {
    Self {
      event_bus,
      worker: None,
      health: EventBusHealthCheck::Disabled,
    }
  }
  pub(super) fn jetstream(
    event_bus: Arc<EventBus>,
    worker: JetStreamIncomingConsumer,
    health: nats_client::health::HealthCheck,
  ) -> Self {
    Self {
      event_bus,
      worker: Some(worker),
      health: EventBusHealthCheck::JetStream(health),
    }
  }

  pub fn event_bus(&self) -> Arc<EventBus> {
    Arc::clone(&self.event_bus)
  }

  pub(crate) fn health_check(&self) -> EventBusHealthCheck {
    self.health.clone()
  }

  pub async fn run(self) -> Result<(), EventBusRuntimeError> {
    let Some(worker) = self.worker else {
      // У локального Event Bus нет фонового worker-а. Оставляем runtime в ожидании,
      // чтобы supervisor не воспринял отсутствие worker-а как завершение подсистемы
      // и не остановил приложение.
      return pending().await;
    };

    worker.run().await.map_err(EventBusRuntimeError::Consumer)
  }
}
