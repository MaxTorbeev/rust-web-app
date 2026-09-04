use std::collections::BTreeSet;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PresenceClientIdPolicy {
  /// Соединение не идентифицировано и не может изменять Presence.
  Unidentified,

  /// Разрешены только перечисленные client ID.
  Bound(BTreeSet<String>),

  /// Разрешён любой client ID.
  Any,
}

impl PresenceClientIdPolicy {
  pub fn allows(&self, client_id: &str) -> bool {
    match self {
      Self::Unidentified => false,
      Self::Bound(client_ids) => client_ids.contains(client_id),
      Self::Any => true,
    }
  }
}
