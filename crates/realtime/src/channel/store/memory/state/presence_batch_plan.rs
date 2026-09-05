use crate::{
  CommittedChannelTransition, CommittedPresenceEvent, OccupancyChange, PresenceBatchCommand,
  PresenceBatchItem, PresenceChangeAction, PresenceChannelChanged, PresenceMember,
  PresenceMemberChange, PresenceMutationAction, PresenceRejection,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Проверенный, но ещё не применённый batch Presence-действий одного соединения.
///
/// Строится на рабочей копии участников соединения: элементы применяются
/// последовательно, а состояние канала не меняется, пока все элементы не
/// пройдут проверки. Затем `ChannelState` фиксирует план целиком.
pub(super) struct PresenceBatchPlan {
  /// Участники соединения после применения всех элементов.
  members: HashMap<String, PresenceMember>,

  /// Canonical deltas в порядке элементов batch.
  member_changes: Vec<PresenceMemberChange>,

  /// Ревизия Presence, которую получит зафиксированный batch.
  presence_revision: u64,
}

impl PresenceBatchPlan {
  /// Строит план поверх текущих участников соединения.
  ///
  /// Доменный отказ возвращается как `Err(PresenceRejection)`; план при этом
  /// отбрасывается, состояние канала не затронуто.
  pub(super) fn build(
    command: &PresenceBatchCommand,
    current_members: HashMap<String, PresenceMember>,
    presence_revision: u64,
  ) -> Result<Self, PresenceRejection> {
    let mut plan = Self {
      members: current_members,
      member_changes: Vec::with_capacity(command.items.len()),
      presence_revision,
    };

    for (index, item) in command.items.iter().enumerate() {
      plan.apply_item(command, index, item)?;
    }

    Ok(plan)
  }

  /// Число участников соединения после фиксации плана.
  pub(super) fn member_count(&self) -> usize {
    self.members.len()
  }

  /// Применяет один элемент batch к рабочей копии участников.
  fn apply_item(
    &mut self,
    command: &PresenceBatchCommand,
    index: usize,
    item: &PresenceBatchItem,
  ) -> Result<(), PresenceRejection> {
    let actor = &command.actor.connection_actor;

    let client_id = item
      .client_id
      .as_deref()
      .ok_or(PresenceRejection::UnidentifiedConnection)?;

    if !command.actor.client_id_policy.allows(client_id) {
      return Err(PresenceRejection::ClientIdNotAllowed {
        client_id: client_id.to_owned(),
      });
    }

    let action = Self::resolve_action(item.action, self.members.contains_key(client_id))?;
    let message_id = command.message_id(index);

    let data = match action {
      PresenceChangeAction::Enter | PresenceChangeAction::Update => {
        self.members.insert(
          client_id.to_owned(),
          PresenceMember {
            connection_id: actor.connection_id.clone(),
            client_id: client_id.to_owned(),
            node_instance: actor.node_instance.clone(),
            data: item.data.clone(),
            last_message_id: message_id.clone(),
            presence_revision: self.presence_revision,
            updated_at_ms: command.request_time.as_millis(),
          },
        );

        item.data.clone()
      }
      PresenceChangeAction::Leave => {
        let previous = self.members.remove(client_id);

        Self::leave_data(item, previous)
      }
    };

    self.member_changes.push(PresenceMemberChange {
      action,
      connection_id: actor.connection_id.clone(),
      client_id: client_id.to_owned(),
      data,
      message_id,
      timestamp: command.request_time,
    });

    Ok(())
  }

  /// Сопоставляет клиентское действие с canonical delta по текущему
  /// присутствию участника.
  ///
  /// Повторный `ENTER` уже присутствующего участника обновляет его данные.
  /// `UPDATE` и `LEAVE` отсутствующего участника — доменный отказ.
  fn resolve_action(
    action: PresenceMutationAction,
    is_present: bool,
  ) -> Result<PresenceChangeAction, PresenceRejection> {
    match (action, is_present) {
      (PresenceMutationAction::Enter, false) => Ok(PresenceChangeAction::Enter),
      (PresenceMutationAction::Enter, true) | (PresenceMutationAction::Update, true) => {
        Ok(PresenceChangeAction::Update)
      }
      (PresenceMutationAction::Leave, true) => Ok(PresenceChangeAction::Leave),
      (PresenceMutationAction::Update, false) | (PresenceMutationAction::Leave, false) => {
        Err(PresenceRejection::InvalidMemberState)
      }
    }
  }

  /// `LEAVE` публикует данные из сообщения, если они есть, иначе последние
  /// данные участника.
  fn leave_data(
    item: &PresenceBatchItem,
    previous: Option<PresenceMember>,
  ) -> Option<serde_json::Value> {
    item
      .data
      .clone()
      .or_else(|| previous.and_then(|member| member.data))
  }

  /// Разбирает план на участников для записи в состояние и delta для события.
  ///
  /// Событие собирается после записи участников: только тогда известны
  /// итоговые метрики Occupancy канала.
  pub(super) fn into_parts(self) -> (HashMap<String, PresenceMember>, PresenceBatchDelta) {
    let delta = PresenceBatchDelta {
      member_changes: self.member_changes,
      presence_revision: self.presence_revision,
    };

    (self.members, delta)
  }
}

/// Canonical deltas зафиксированного batch, ожидающие сборки события.
pub(super) struct PresenceBatchDelta {
  member_changes: Vec<PresenceMemberChange>,
  presence_revision: u64,
}

impl PresenceBatchDelta {
  pub(super) const fn presence_revision(&self) -> u64 {
    self.presence_revision
  }

  /// Формирует transition из deltas и итогового состояния канала.
  pub(super) fn into_transition(
    self,
    command: &PresenceBatchCommand,
    occupancy_version: u64,
    occupancy: Option<OccupancyChange>,
  ) -> CommittedChannelTransition {
    let event = CommittedPresenceEvent::new(
      Uuid::new_v4(),
      PresenceChannelChanged {
        channel: command.channel.clone(),
        origin: command.actor.connection_actor.node_instance.clone(),
        presence_revision: Some(self.presence_revision),
        occupancy_version,
        member_changes: self.member_changes,
        occupancy,
        occurred_at: command.request_time,
      },
    );

    CommittedChannelTransition::Changed(event)
  }
}
