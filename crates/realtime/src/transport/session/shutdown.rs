use tokio::sync::watch;

#[derive(Clone)]
pub struct ShutdownTrigger {
  sender: watch::Sender<bool>,
}

pub struct ShutdownListener {
  receiver: watch::Receiver<bool>,
}

impl ShutdownTrigger {
  fn request(&self) {
    self.sender.send_replace(true);
  }
}

impl ShutdownListener {
  pub fn new(receiver: watch::Receiver<bool>) -> Self {
    Self {
      receiver
    }
  }

  pub(crate) async fn requested(&mut self) -> bool {
    self
      .receiver
      .wait_for(|requested| *requested)
      .await
      .is_ok()
  }
}
