use std::sync::Arc;
use crate::{PresenceCommitDelivery, PresenceStore};

pub struct PresenceService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn PresenceCommitDelivery>,
}