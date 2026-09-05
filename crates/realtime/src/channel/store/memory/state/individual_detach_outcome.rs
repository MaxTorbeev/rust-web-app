use crate::{OccupancyChange, PresenceMember};

/// Результат удаления индивидуального attachment.
pub(super) enum IndividualDetachOutcome {
  NotAttached {
    occupancy_version: u64,
  },
  Detached {
    removed_members: Vec<PresenceMember>,
    presence_revision: Option<u64>,
    occupancy_version: u64,
    occupancy_change: OccupancyChange,
  },
}
