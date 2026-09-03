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