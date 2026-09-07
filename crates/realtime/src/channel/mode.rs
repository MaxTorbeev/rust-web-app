use crate::ProtocolFlag;
use serde::{Deserialize, Serialize};

/// Режим работы соединения с каналом.
///
/// Сериализуется camelCase-именем (`subscribe`, `presenceSubscribe`, …) —
/// так режимы хранятся в attachment.
#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelMode {
  Subscribe,
  Publish,
  Presence,
  PresenceSubscribe,
}

impl ChannelMode {
  pub const ALL: [Self; 4] = [
    Self::Subscribe,
    Self::Publish,
    Self::Presence,
    Self::PresenceSubscribe,
  ];

  /// Операция capability токена, необходимая для режима.
  ///
  /// Подписка на Presence покрывается операцией `subscribe`.
  pub const fn capability_operation(self) -> &'static str {
    match self {
      Self::Subscribe | Self::PresenceSubscribe => "subscribe",
      Self::Publish => "publish",
      Self::Presence => "presence",
    }
  }

  pub const fn flag(self) -> ProtocolFlag {
    match self {
      Self::Subscribe => ProtocolFlag::SUBSCRIBE,
      Self::Publish => ProtocolFlag::PUBLISH,
      Self::Presence => ProtocolFlag::PRESENCE,
      Self::PresenceSubscribe => ProtocolFlag::PRESENCE_SUBSCRIBE,
    }
  }

  /// Режимы, запрошенные клиентом в `ATTACH.flags`.
  pub fn from_flags(flags: ProtocolFlag) -> Vec<Self> {
    Self::ALL
      .into_iter()
      .filter(|mode| flags.contains(mode.flag()))
      .collect()
  }

  /// Флаги effective modes для `ATTACHED.flags`.
  pub fn to_flags(modes: &[Self]) -> ProtocolFlag {
    modes
      .iter()
      .fold(ProtocolFlag::empty(), |flags, mode| flags | mode.flag())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn modes_wire_format() {
    insta::assert_json_snapshot!("channel_modes", ChannelMode::ALL);
  }
}
