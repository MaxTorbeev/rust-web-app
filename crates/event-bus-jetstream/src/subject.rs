use event_bus::DeliveryClass;
use support::app::APP_NAMESPACE_SEPARATOR;

use crate::error::EventSubjectError;

pub(crate) fn event_subject(
  prefix: &str,
  event_name: &str,
  delivery: DeliveryClass,
) -> Result<String, EventSubjectError> {
  if !is_valid_event_name(event_name) {
    return Err(EventSubjectError::InvalidEventName {
      event_name: event_name.to_owned(),
    });
  }

  let delivery = match delivery {
    DeliveryClass::AllNodes => "all",
    DeliveryClass::WorkQueue => "work",
    DeliveryClass::LocalOnly => {
      return Err(EventSubjectError::UnsupportedDeliveryClass);
    }
  };

  Ok(format!(
    "{prefix}{APP_NAMESPACE_SEPARATOR}{delivery}{APP_NAMESPACE_SEPARATOR}{event_name}"
  ))
}

pub(crate) fn is_valid_subject_token(value: &str) -> bool {
  !value.is_empty()
    && value
      .bytes()
      .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
}

fn is_valid_event_name(event_name: &str) -> bool {
  !event_name.is_empty() && event_name.split('.').all(is_valid_subject_token)
}
