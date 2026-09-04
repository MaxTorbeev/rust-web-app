use crate::PresenceAction;

/// A client action that may mutate authoritative Presence state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresenceMutationAction {
  Enter,
  Leave,
  Update,
}

impl PresenceMutationAction {
  /// Возвращает стабильное имя действия для нормализации команды.
  ///
  /// Значение участвует в хеше запроса. Его изменение требует смены версии
  /// формата журнала операций, иначе одинаковые команды получат разные хеши.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Enter => "enter",
      Self::Update => "update",
      Self::Leave => "leave",
    }
  }
}

impl TryFrom<PresenceAction> for PresenceMutationAction {
  type Error = PresenceAction;

  fn try_from(action: PresenceAction) -> Result<Self, Self::Error> {
    match action {
      PresenceAction::Enter => Ok(Self::Enter),
      PresenceAction::Leave => Ok(Self::Leave),
      PresenceAction::Update => Ok(Self::Update),
      PresenceAction::Absent | PresenceAction::Present => Err(action),
    }
  }
}

impl From<PresenceMutationAction> for PresenceAction {
  fn from(action: PresenceMutationAction) -> Self {
    match action {
      PresenceMutationAction::Enter => Self::Enter,
      PresenceMutationAction::Leave => Self::Leave,
      PresenceMutationAction::Update => Self::Update,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_wire_mutation_actions() {
    assert_eq!(
      PresenceMutationAction::try_from(PresenceAction::Enter),
      Ok(PresenceMutationAction::Enter),
    );
    assert_eq!(
      PresenceMutationAction::try_from(PresenceAction::Update),
      Ok(PresenceMutationAction::Update),
    );
    assert_eq!(
      PresenceMutationAction::try_from(PresenceAction::Leave),
      Ok(PresenceMutationAction::Leave),
    );
  }

  #[test]
  fn rejects_wire_state_actions() {
    assert_eq!(
      PresenceMutationAction::try_from(PresenceAction::Absent),
      Err(PresenceAction::Absent),
    );
    assert_eq!(
      PresenceMutationAction::try_from(PresenceAction::Present),
      Err(PresenceAction::Present),
    );
  }
}
