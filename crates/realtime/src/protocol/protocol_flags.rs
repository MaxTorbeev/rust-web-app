use bitflags::bitflags;

bitflags! {
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
  pub struct ProtocolFlag: u64 {
    /// Signals that the client must wait for the initial presence `SYNC`
    /// before treating the channel's presence set as complete.
    const HAS_PRESENCE = 1 << 0;

    /// Requested (`ATTACH`) or effective (`ATTACHED`) channel modes.
    const PRESENCE = 1 << 16;
    const PUBLISH = 1 << 17;
    const SUBSCRIBE = 1 << 18;
    const PRESENCE_SUBSCRIBE = 1 << 19;
  }
}

impl ProtocolFlag {
  /// Биты, обозначающие channel modes.
  pub const MODES: Self = Self::PRESENCE
    .union(Self::PUBLISH)
    .union(Self::SUBSCRIBE)
    .union(Self::PRESENCE_SUBSCRIBE);

  /// Читает флаги из wire-значения, отбрасывая неизвестные биты.
  pub fn from_wire(flags: Option<u64>) -> Self {
    Self::from_bits_truncate(flags.unwrap_or(0))
  }
}
