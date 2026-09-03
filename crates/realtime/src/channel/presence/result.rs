use crate::channel::attachment::Attachment;
use crate::channel::presence::snapshot::PresenceSnapshot;
use crate::CommittedTransition;

pub struct PresenceAttachResult {
  pub attachment: Attachment,
  pub snapshot: PresenceSnapshot,
  pub transition: CommittedTransition,
}