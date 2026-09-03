use crate::OccupancyCategory;

#[derive(Debug, Clone)]
pub enum OccupancySubscription {
  Metrics,
  Category(OccupancyCategory),
  Categories(Vec<OccupancyCategory>),
}
