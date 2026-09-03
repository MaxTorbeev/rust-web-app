use crate::{
  CommittedTransition,
  PresenceSnapshot,
};
use crate::channel::attachment::Attachment;

pub struct PresenceAttachOutcome {
  pub attachment: Attachment,
  pub snapshot: PresenceSnapshot,
  pub transition: CommittedTransition,
}