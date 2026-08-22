use event_bus::DeliveryClass;
use crate::JetStreamPublisherError;

/// Адрес маршрутизации внутри NATS
pub fn event_subject(prefix: &str, event_name: &str, delivery: DeliveryClass) -> Result<String, JetStreamPublisherError> {
  let delivery = match delivery {
    DeliveryClass::AllNodes => "all",
    DeliveryClass::WorkQueue => "work",
    DeliveryClass::LocalOnly => {
      return Err(
        JetStreamPublisherError::UnsupportedDeliveryClass
      );
    }
  };

  Ok(format!("{}.{}.{}", prefix, delivery, event_name))
}