#[derive(Debug, Clone)]
pub enum ChannelMode {
  Subscribe,
  Publish,
  Presence,
  PresenceSubscribe,
}
