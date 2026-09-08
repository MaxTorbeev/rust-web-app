use crate::outcome::Fence;

#[test]
fn fence_orders_by_value() {
  assert!(Fence::new(1) < Fence::new(2));
  assert_eq!(Fence::new(5).get(), 5);
}
