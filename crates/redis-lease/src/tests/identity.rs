use crate::error::RedisLeaseError;
use crate::identity::{LeaseKey, LeaseOwner};

#[test]
fn empty_key_and_owner_are_rejected() {
  assert!(matches!(
    LeaseKey::new(""),
    Err(RedisLeaseError::Empty { field: "key" })
  ));
  assert!(matches!(
    LeaseOwner::new(""),
    Err(RedisLeaseError::Empty { field: "owner" })
  ));
}

#[test]
fn debug_output_hides_the_shared_representation() {
  // Общий внутренний тип — деталь реализации; в логах должны быть роли.
  let key = LeaseKey::new("k").unwrap();
  let owner = LeaseOwner::new("o").unwrap();

  assert_eq!(format!("{key:?}"), r#"LeaseKey("k")"#);
  assert_eq!(format!("{owner:?}"), r#"LeaseOwner("o")"#);
}
