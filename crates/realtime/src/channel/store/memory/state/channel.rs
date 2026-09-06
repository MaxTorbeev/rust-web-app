use std::collections::HashMap;
use crate::{Attachment, ChannelMode, ChannelStateStoreError, CommittedChannelTransition, ConnectionId, OccupancyChange, OccupancyMetrics, PresenceBatchCommand, PresenceMember, PresenceMutationOutcome, PresenceRejection, PresenceSnapshot};
use crate::connection::ConnectionActor;
use super::{IndividualDetachOutcome, PresenceBatchPlan};

/// Внутреннее состояние одного канала в локальном хранилище.
#[derive(Default)]
pub(super) struct ChannelState {
  /// Активные присоединения к этому каналу по идентификатору соединения.
  attachments: HashMap<ConnectionId, Attachment>,

  /// Участники Presence, сгруппированные по соединению и `client_id`.
  members: HashMap<ConnectionId, HashMap<String, PresenceMember>>,

  /// Текущая ревизия списка участников Presence.
  presence_revision: u64,

  /// Текущая версия метрик Occupancy.
  occupancy_version: u64,
}

impl ChannelState {
  /// Рассчитывает текущие метрики Occupancy канала.
  pub(super) fn occupancy(&self) -> OccupancyMetrics {
    let mut metrics = OccupancyMetrics {
      connections: 0,
      publishers: 0,
      subscribers: 0,
      presence_connections: 0,
      presence_subscribers: 0,
      presence_members: self
        .members
        .values()
        .map(|members| members.len() as u64)
        .sum(),
    };

    for attachment in self.attachments.values() {
      // Локальное хранилище принимает только индивидуальный учёт: каждый
      // attachment — ровно одно соединение в метриках.
      debug_assert!(attachment.is_individual(), "memory store holds only individual attachments");

      metrics.connections += 1;

      if attachment.has_mode(ChannelMode::Publish) {
        metrics.publishers += 1;
      }

      if attachment.has_mode(ChannelMode::Subscribe) {
        metrics.subscribers += 1;
      }

      if attachment.has_mode(ChannelMode::Presence) {
        metrics.presence_connections += 1;
      }

      if attachment.has_mode(ChannelMode::PresenceSubscribe) {
        metrics.presence_subscribers += 1;
      }
    }

    metrics
  }

  /// Создаёт snapshot текущего состояния Presence и Occupancy.
  pub(super) fn snapshot(&self) -> PresenceSnapshot {
    let members = self
      .members
      .values()
      .flat_map(|members| members.values())
      .cloned()
      .collect();

    PresenceSnapshot {
      members,
      presence_revision: self.presence_revision,
      occupancy_version: self.occupancy_version,
      occupancy: self.occupancy(),
    }
  }

  /// Сохраняет или обновляет attachment соединения.
  ///
  /// Возвращает изменение Occupancy, если сохранение повлияло на метрики канала.
  pub(super) fn save_attachment(
    &mut self,
    attachment: Attachment,
  ) -> Result<Option<OccupancyChange>, ChannelStateStoreError> {
    self.validate_attachment(&attachment)?;

    // Переполнение проверяется до изменения состояния, как и в остальных
    // операциях: после проверки фиксация не может завершиться ошибкой.
    let next_occupancy_version = self.next_occupancy_version()?;

    let before = self.occupancy();

    self
      .attachments
      .insert(attachment.connection_id.clone(), attachment);

    let change = OccupancyChange::between(before, self.occupancy());

    if change.is_some() {
      self.occupancy_version = next_occupancy_version;
    }

    Ok(change)
  }

  /// Применяет batch клиентских Presence-действий одного соединения.
  ///
  /// Batch либо фиксируется целиком одной ревизией и одним событием, либо
  /// отклоняется без изменений. Доменный отказ возвращается как
  /// `Ok(Rejected)`, инфраструктурная ошибка — как `Err`.
  pub(super) fn apply_presence_batch(
    &mut self,
    command: &PresenceBatchCommand,
  ) -> Result<PresenceMutationOutcome, ChannelStateStoreError> {
    if command.items.is_empty() {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "presence batch must contain at least one item".to_owned(),
      });
    }

    let actor = &command.actor.connection_actor;

    if let Err(rejection) = self.check_presence_attachment(actor)? {
      return Ok(PresenceMutationOutcome::Rejected(rejection));
    }

    let current_members = self
      .members
      .get(&actor.connection_id)
      .cloned()
      .unwrap_or_default();
    let current_count = current_members.len();

    let plan = match PresenceBatchPlan::build(command, current_members, self.next_presence_revision()?) {
      Ok(plan) => plan,
      Err(rejection) => return Ok(PresenceMutationOutcome::Rejected(rejection)),
    };

    // Occupancy меняется только вместе с числом участников; переполнение
    // проверяется до первого изменения состояния.
    let next_occupancy_version = if plan.member_count() != current_count {
      Some(self.next_occupancy_version()?)
    } else {
      None
    };

    Ok(PresenceMutationOutcome::Committed(
      self.commit_presence_batch(command, plan, next_occupancy_version),
    ))
  }

  /// Проверяет, что соединение может изменять Presence этого канала.
  ///
  /// Внешний `Err` — нарушение владения attachment (инфраструктура),
  /// внутренний — доменный отказ клиенту.
  fn check_presence_attachment(
    &self,
    actor: &ConnectionActor,
  ) -> Result<Result<(), PresenceRejection>, ChannelStateStoreError> {
    let Some(attachment) = self.attachments.get(&actor.connection_id) else {
      return Ok(Err(PresenceRejection::NotAttached));
    };

    Self::validate_individual_detach(attachment, actor)?;

    if !attachment.has_mode(ChannelMode::Presence) {
      return Ok(Err(PresenceRejection::PresenceModeNotEnabled));
    }

    Ok(Ok(()))
  }

  /// Фиксирует проверенный план: участники, ревизии, Occupancy и событие.
  ///
  /// Все проверки уже выполнены; этот шаг не может завершиться ошибкой.
  fn commit_presence_batch(
    &mut self,
    command: &PresenceBatchCommand,
    plan: PresenceBatchPlan,
    next_occupancy_version: Option<u64>,
  ) -> CommittedChannelTransition {
    let connection_id = &command.actor.connection_actor.connection_id;
    let before = self.occupancy();
    let (members, delta) = plan.into_parts();

    if members.is_empty() {
      self.members.remove(connection_id);
    } else {
      self.members.insert(connection_id.clone(), members);
    }

    self.presence_revision = delta.presence_revision();

    if let Some(version) = next_occupancy_version {
      self.occupancy_version = version;
    }

    let occupancy = OccupancyChange::between(before, self.occupancy());

    delta.into_transition(command, self.occupancy_version, occupancy)
  }

  pub(super) fn detach_individual(&mut self, actor: &ConnectionActor) -> Result<IndividualDetachOutcome, ChannelStateStoreError> {
    // Находим attachment; его отсутствие означает успешный detach без изменений.
    let Some(attachment) = self.attachments.get(&actor.connection_id) else {
      return Ok(IndividualDetachOutcome::NotAttached {
        occupancy_version: self.occupancy_version,
      });
    };

    // Проверяем принадлежность attachment экземпляру ноды и индивидуальный учёт.
    Self::validate_individual_detach(attachment, actor)?;

    // Определяем, затронет ли detach список участников Presence.
    let has_members = self
      .members
      .get(&actor.connection_id)
      .is_some_and(|members| !members.is_empty());

    // Готовим новую ревизию Presence только при наличии участников для удаления.
    // Обе версии проверяем на переполнение до изменения состояния.
    let next_presence_revision = if has_members {
      Some(self.next_presence_revision()?)
    } else {
      None
    };
    // Удаление индивидуального attachment всегда меняет Occupancy.
    let next_occupancy_version = self.next_occupancy_version()?;

    // Сохраняем метрики до удаления для расчёта изменения Occupancy.
    let before = self.occupancy();

    // Отсоединяем соединение от канала.
    self.attachments.remove(&actor.connection_id);
    // Удаляем всех Presence-участников соединения в детерминированном порядке.
    let removed_members = self.remove_members(&actor.connection_id);

    // Фиксируем ревизию изменённого списка участников Presence.
    if let Some(revision) = next_presence_revision {
      self.presence_revision = revision;
    }
    // Фиксируем новую версию метрик Occupancy.
    self.occupancy_version = next_occupancy_version;

    // Рассчитываем метрики после удаления и изменение Occupancy.
    // Удаление индивидуального attachment уменьшает `connections`, поэтому
    // изменение есть всегда; его отсутствие — нарушение инварианта `occupancy()`.
    // Состояние канала к этому моменту уже согласовано, теряется только событие.
    let after = self.occupancy();
    let occupancy_change = OccupancyChange::between(before, after).ok_or_else(|| {
      ChannelStateStoreError::Internal {
        message: format!(
          "detaching individual attachment {} did not change occupancy",
          actor.connection_id.as_str(),
        ),
      }
    })?;

    // Возвращаем изменения канала для формирования события detach.
    Ok(IndividualDetachOutcome::Detached {
      removed_members,
      presence_revision: next_presence_revision,
      occupancy_version: next_occupancy_version,
      occupancy_change,
    })
  }


  /// Проверяет, что индивидуальный detach можно выполнить без ошибки.
  ///
  /// Используется перед отключением соединения сразу от нескольких каналов.
  pub(super) fn check_individual_detach(
    &self,
    actor: &ConnectionActor,
  ) -> Result<(), ChannelStateStoreError> {
    let Some(attachment) = self.attachments.get(&actor.connection_id) else {
      return Ok(());
    };

    Self::validate_individual_detach(attachment, actor)?;

    let has_members = self
      .members
      .get(&actor.connection_id)
      .is_some_and(|members| !members.is_empty());

    if has_members {
      self.next_presence_revision()?;
    }

    self.next_occupancy_version()?;

    Ok(())
  }

  fn validate_individual_detach(attachment: &Attachment, actor: &ConnectionActor) -> Result<(), ChannelStateStoreError> {
    if attachment.node_instance != actor.node_instance {
      return Err(ChannelStateStoreError::Conflict {
        message: format!(
          "attachment {} belongs to another node instance",
          actor.connection_id.as_str(),
        ),
      });
    }

    if !attachment.is_individual() {
      return Err(ChannelStateStoreError::Internal {
        message: "aggregated attachment cannot be removed as individual".to_owned(),
      });
    }

    Ok(())
  }

  fn next_presence_revision(&self) -> Result<u64, ChannelStateStoreError> {
    self
      .presence_revision
      .checked_add(1)
      .ok_or_else(|| ChannelStateStoreError::Internal {
        message: "presence revision overflow".to_owned(),
      })
  }

  fn next_occupancy_version(&self) -> Result<u64, ChannelStateStoreError> {
    self
      .occupancy_version
      .checked_add(1)
      .ok_or_else(|| ChannelStateStoreError::Internal {
        message: "occupancy version overflow".to_owned(),
      })
  }

  fn remove_members(&mut self, connection_id: &ConnectionId) -> Vec<PresenceMember> {
    let mut removed_members = self
      .members
      .remove(connection_id)
      .unwrap_or_default()
      .into_values()
      .collect::<Vec<_>>();

    // HashMap не гарантирует порядок. Событие должно быть детерминированным.
    removed_members.sort_by(|left, right| left.client_id.cmp(&right.client_id));

    removed_members
  }

  fn validate_attachment(&self, attachment: &Attachment) -> Result<(), ChannelStateStoreError> {
    let current = self.attachments.get(&attachment.connection_id);

    if current.is_some_and(|current| current.node_instance != attachment.node_instance) {
      return Err(ChannelStateStoreError::Conflict {
        message: format!("attachment {} belongs to another node instance", attachment.connection_id.as_str()),
      });
    }

    Ok(())
  }
}
