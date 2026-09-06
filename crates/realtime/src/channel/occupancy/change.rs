use crate::{OccupancyCategory, OccupancyMetrics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupancyChange {
  pub metrics: OccupancyMetrics,
  pub changed_categories: BTreeSet<OccupancyCategory>,
  pub zero_boundary_categories: BTreeSet<OccupancyCategory>,
}

impl OccupancyChange {

  /// Сравнивает исходные и новые метрики Occupancy.
  ///
  /// Возвращает `None`, если значения всех категорий остались прежними.
  /// Иначе возвращает [`OccupancyChange`] с новыми метриками, перечнем
  /// изменившихся категорий и категориями, значение которых перешло
  /// с нулевого на ненулевое или обратно.
  pub fn between(before: OccupancyMetrics, after: OccupancyMetrics) -> Option<Self> {
    let mut changed_categories = BTreeSet::new();
    let mut zero_boundary_categories = BTreeSet::new();

    for ((category, before), (_, after)) in before.entries().zip(after.entries()) {
      if before == after {
        continue;
      }

      changed_categories.insert(category.clone());

      if before == 0 || after == 0 {
        zero_boundary_categories.insert(category);
      }
    }

    if changed_categories.is_empty() {
      return None;
    }

    Some(Self {
      metrics: after,
      changed_categories,
      zero_boundary_categories,
    })
  }
}