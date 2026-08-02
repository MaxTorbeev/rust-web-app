use bitflags::bitflags;

bitflags! {
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
  pub struct ProtocolFlag: u64 {
    /// Signals that the client must wait for the initial presence `SYNC`
    /// before treating the channel's presence set as complete.
    const HAS_PRESENCE = 1 << 0;
  }
}


// #[cfg(test)]
// mod tests {
//   use super::*;
//   #[test]
//
// }
