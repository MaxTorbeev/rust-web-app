#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ChannelMode {
  Subscribe,
  Publish,
  Presence,
  PresenceSubscribe,
}
