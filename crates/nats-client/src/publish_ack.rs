use async_nats::jetstream::publish::PublishAck as DriverPublishAck;

/// JetStream confirmation that a message was accepted by a stream.
///
/// It does not confirm consumer processing or delivery to an application.
///
/// Подтверждение JetStream о принятии публикации в поток. Оно не означает, что
/// consumer обработал сообщение или что данные доставлены приложению.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishAck {
  /// Stream that accepted the message.
  ///
  /// Поток, принявший сообщение.
  pub stream: String,

  /// Stored stream sequence assigned to the message.
  ///
  /// Порядковый номер, присвоенный сообщению внутри потока.
  pub sequence: u64,

  /// JetStream domain that produced the acknowledgment.
  ///
  /// Домен JetStream, вернувший подтверждение.
  pub domain: String,

  /// Whether JetStream recognized the message ID as a duplicate.
  ///
  /// Признак того, что JetStream распознал `Nats-Msg-Id` как дубликат.
  pub duplicate: bool,

  /// Optional server value used by specialized JetStream streams.
  ///
  /// Дополнительное значение сервера для специализированных типов потоков.
  pub value: Option<String>,
}

impl PublishAck {
  pub(crate) fn from_driver(ack: DriverPublishAck) -> Self {
    Self {
      stream: ack.stream,
      sequence: ack.sequence,
      domain: ack.domain,
      duplicate: ack.duplicate,
      value: ack.value,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_driver_ack_without_exposing_it() {
    let ack = PublishAck::from_driver(DriverPublishAck {
      stream: "EVENTS".to_owned(),
      sequence: 42,
      domain: "production".to_owned(),
      duplicate: true,
      value: Some("7".to_owned()),
    });

    assert_eq!(
      ack,
      PublishAck {
        stream: "EVENTS".to_owned(),
        sequence: 42,
        domain: "production".to_owned(),
        duplicate: true,
        value: Some("7".to_owned()),
      },
    );
  }
}
