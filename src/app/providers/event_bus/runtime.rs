use std::future::pending;
use std::sync::Arc;
use event_bus::EventBus;
use event_bus_jetstream::JetStreamIncomingConsumer;
use super::{EventBusRuntimeError};

pub struct EventBusRuntime {
  event_bus: Arc<EventBus>,
  worker: Option<JetStreamIncomingConsumer>
}

impl EventBusRuntime {
  pub (super) fn local(event_bus: Arc<EventBus>) -> Self {
    Self {
      event_bus,
      worker: None
    }
  }
  pub (super) fn jetstream(event_bus: Arc<EventBus>, worker: JetStreamIncomingConsumer) -> Self {
    Self {
      event_bus,
      worker: Some(worker)
    }
  }

  pub fn event_bus(&self) -> Arc<EventBus> {
    Arc::clone(&self.event_bus)
  }

  pub async fn run(mut self) -> Result<(), EventBusRuntimeError> {
    let Some(worker) = &mut self.worker else {
      // У локального Event Bus нет фонового worker-а. Оставляем runtime в ожидании,
      // чтобы supervisor не воспринял отсутствие worker-а как завершение подсистемы
      // и не остановил приложение.
      return pending().await;
    };

    worker
      .run()
      .await
      .map_err(EventBusRuntimeError::Consumer)
  }
}
