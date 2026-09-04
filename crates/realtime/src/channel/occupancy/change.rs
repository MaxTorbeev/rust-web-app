use crate::{OccupancyCategory, OccupancyMetrics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OccupancyChange {
  pub metrics: OccupancyMetrics,
  pub changed_categories: BTreeSet<OccupancyCategory>,
  pub zero_boundary_categories: BTreeSet<OccupancyCategory>,
}
