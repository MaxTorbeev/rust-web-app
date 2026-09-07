pub mod attachment;
pub mod fixtures;
pub mod ledger;
pub mod ledger_model;
pub mod presence;
pub mod snapshot;

use realtime::{AttachmentStore, PresenceStore};

/// Хранилище, проходящее contract suite: обе половины контракта на одном типе.
pub trait ContractStore: AttachmentStore + PresenceStore {}

impl<S: AttachmentStore + PresenceStore> ContractStore for S {}
