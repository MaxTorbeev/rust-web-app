use crate::OccupancyCategory;
use thiserror::Error;

/// Параметр `ATTACH.params.occupancy`, требующий capability `channel-metadata`.
pub const OCCUPANCY_PARAM: &str = "occupancy";

/// Операция capability, необходимая для подписки на Occupancy.
pub const OCCUPANCY_CAPABILITY_OPERATION: &str = "channel-metadata";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OccupancySubscription {
  Metrics,
  Category(OccupancyCategory),
  Categories(Vec<OccupancyCategory>),
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("unsupported occupancy subscription `{value}`")]
pub struct OccupancySubscriptionError {
  pub value: String,
}

impl OccupancySubscription {
  /// Разбирает wire-значение: `metrics` либо одна или несколько
  /// `metrics.<category>` через запятую.
  ///
  /// Неподдерживаемая категория (например, `objectPublishers`) отклоняется,
  /// а не превращается в ложный ноль.
  pub fn parse(value: &str) -> Result<Self, OccupancySubscriptionError> {
    let unsupported = || OccupancySubscriptionError {
      value: value.to_owned(),
    };

    let mut categories = Vec::new();

    for part in value.split(',') {
      let part = part.trim();

      if part == "metrics" {
        // `metrics` покрывает все категории; сочетать его с частями бессмысленно.
        if value.contains(',') {
          return Err(unsupported());
        }

        return Ok(Self::Metrics);
      }

      let category = part
        .strip_prefix("metrics.")
        .and_then(OccupancyCategory::from_wire_name)
        .ok_or_else(unsupported)?;

      if !categories.contains(&category) {
        categories.push(category);
      }
    }

    match categories.len() {
      0 => Err(unsupported()),
      1 => Ok(Self::Category(categories.remove(0))),
      _ => {
        categories.sort();
        Ok(Self::Categories(categories))
      }
    }
  }

  /// Canonical wire-значение для `ATTACHED.params`.
  pub fn to_wire_value(&self) -> String {
    match self {
      Self::Metrics => "metrics".to_owned(),
      Self::Category(category) => format!("metrics.{}", category.wire_name()),
      Self::Categories(categories) => categories
        .iter()
        .map(|category| format!("metrics.{}", category.wire_name()))
        .collect::<Vec<_>>()
        .join(","),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_supported_values() {
    assert_eq!(OccupancySubscription::parse("metrics"), Ok(OccupancySubscription::Metrics));
    assert_eq!(
      OccupancySubscription::parse("metrics.presenceMembers"),
      Ok(OccupancySubscription::Category(OccupancyCategory::PresenceMembers)),
    );
    assert_eq!(
      OccupancySubscription::parse("metrics.subscribers, metrics.connections"),
      Ok(OccupancySubscription::Categories(vec![
        OccupancyCategory::Connections,
        OccupancyCategory::Subscribers,
      ])),
    );
  }

  #[test]
  fn rejects_unsupported_values() {
    assert!(OccupancySubscription::parse("").is_err());
    assert!(OccupancySubscription::parse("metrics.objectPublishers").is_err());
    assert!(OccupancySubscription::parse("metrics,metrics.connections").is_err());
    assert!(OccupancySubscription::parse("connections").is_err());
  }

  #[test]
  fn canonical_value_round_trips() {
    for value in ["metrics", "metrics.publishers", "metrics.connections,metrics.subscribers"] {
      let parsed = OccupancySubscription::parse(value).unwrap();
      assert_eq!(parsed.to_wire_value(), value);
    }
  }
}
