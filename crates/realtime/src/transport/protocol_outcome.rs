use crate::ProtocolMessage;

pub struct ProtocolOutcome {
  pub replies: Vec<ProtocolMessage>,
  pub disconnect: bool,
}

impl ProtocolOutcome {
  pub fn reply(message: ProtocolMessage) -> Self {
    Self {
      replies: vec![message],
      disconnect: false,
    }
  }

  pub fn replies(messages: Vec<ProtocolMessage>) -> Self {
    Self {
      replies: messages,
      disconnect: false,
    }
  }

  pub fn disconnect(message: ProtocolMessage) -> Self {
    Self {
      replies: vec![message],
      disconnect: true,
    }
  }

  pub fn no_replay() -> Self {
    Self {
      replies: Vec::new(),
      disconnect: false,
    }
  }
}