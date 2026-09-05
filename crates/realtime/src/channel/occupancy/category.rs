use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OccupancyCategory {
  Connections,
  Publishers,
  Subscribers,
  PresenceConnections,
  PresenceSubscribers,
  PresenceMembers,
}

impl OccupancyCategory {
  pub const ALL: [Self; 6] = [
    Self::Connections,
    Self::Publishers,
    Self::Subscribers,
    Self::PresenceConnections,
    Self::PresenceSubscribers,
    Self::PresenceMembers,
  ];

  /// Имя категории в wire contract (`metrics.<name>` и ключи payload).
  pub const fn wire_name(&self) -> &'static str {
    match self {
      Self::Connections => "connections",
      Self::Publishers => "publishers",
      Self::Subscribers => "subscribers",
      Self::PresenceConnections => "presenceConnections",
      Self::PresenceSubscribers => "presenceSubscribers",
      Self::PresenceMembers => "presenceMembers",
    }
  }

  pub fn from_wire_name(name: &str) -> Option<Self> {
    Self::ALL
      .into_iter()
      .find(|category| category.wire_name() == name)
  }
}
