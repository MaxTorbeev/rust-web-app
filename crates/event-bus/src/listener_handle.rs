use crate::EventBusError;

pub struct ListenerHandle {
  inner: tokio_events::SubscriptionHandle
}

impl ListenerHandle {
  pub(crate) fn new(inner: tokio_events::SubscriptionHandle) -> Self {
    Self { inner }
  }
  pub fn detach(self) {
    self.inner.detach();
  }

  pub async fn unsubscribe(self) -> Result<(), EventBusError> {
    self.inner.unsubscribe().await?;

    Ok(())
  }
}