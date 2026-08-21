use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublishReceipt {
    pub event_id: Uuid,
}

impl PublishReceipt {
    pub(crate) fn new(event_id: Uuid) -> Self {
        Self { event_id }
    }
}
