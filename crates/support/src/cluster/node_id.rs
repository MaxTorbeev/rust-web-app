use serde::{Deserialize, Serialize};

/// Stable node id
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NodeId(String);
