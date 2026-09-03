use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use crate::{OccupancyCategory, OccupancyMetrics};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OccupancyChange {
  pub metrics: OccupancyMetrics,
  pub changed_categories: BTreeSet<OccupancyCategory>,
  pub zero_boundary_categories: BTreeSet<OccupancyCategory>,
}