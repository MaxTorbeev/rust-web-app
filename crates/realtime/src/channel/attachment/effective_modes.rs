use crate::{ChannelMode, ProtocolFlag};
use auth::TokenCapability;

/// Вычисляет effective modes attachment-а.
///
/// Effective modes — пересечение запрошенных клиентом режимов и capability
/// токена для канала. Если клиент не запросил режимы, рассматриваются все
/// режимы, разрешённые capability. Пустой результат означает, что у токена нет
/// доступа к каналу.
pub fn resolve_effective_modes(
  capability: &TokenCapability,
  channel: &str,
  requested: ProtocolFlag,
) -> Vec<ChannelMode> {
  let requested_modes = ChannelMode::from_flags(requested);

  let candidates = if requested_modes.is_empty() {
    ChannelMode::ALL.to_vec()
  } else {
    requested_modes
  };

  candidates
    .into_iter()
    .filter(|mode| capability.allows(channel, mode.capability_operation()))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn capability(json: &str) -> TokenCapability {
    json.parse().expect("test capability must be valid")
  }

  #[test]
  fn defaults_to_all_modes_allowed_by_capability() {
    let modes = resolve_effective_modes(
      &capability(r#"{"chat:*": ["subscribe", "presence"]}"#),
      "chat:room",
      ProtocolFlag::empty(),
    );

    assert_eq!(
      modes,
      vec![
        ChannelMode::Subscribe,
        ChannelMode::Presence,
        ChannelMode::PresenceSubscribe
      ],
    );
  }

  #[test]
  fn intersects_requested_modes_with_capability() {
    let modes = resolve_effective_modes(
      &capability(r#"{"chat:*": ["subscribe", "presence"]}"#),
      "chat:room",
      ProtocolFlag::PUBLISH | ProtocolFlag::SUBSCRIBE,
    );

    assert_eq!(modes, vec![ChannelMode::Subscribe]);
  }

  #[test]
  fn denies_channel_outside_capability() {
    let modes = resolve_effective_modes(
      &capability(r#"{"chat:*": ["*"]}"#),
      "admin:room",
      ProtocolFlag::empty(),
    );

    assert!(modes.is_empty());
  }
}
