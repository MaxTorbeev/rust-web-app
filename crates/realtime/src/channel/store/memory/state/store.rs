use std::collections::{HashMap, HashSet};
use support::timestamp::Timestamp;
use uuid::Uuid;
use super::channel::ChannelState;
use super::IndividualDetachOutcome;
use crate::{AttachCommand, AttachmentTracking, ChannelAttachOutcome, ChannelKey, ChannelStateStoreError, CommittedChannelTransition, CommittedPresenceEvent, DetachCommand, PresenceBatchCommand, PresenceChannelChanged, PresenceMutationOutcome, PresenceMutationReceipt, PresenceRejection, PresenceSnapshot};
use crate::channel::presence::{LedgerLookup, PresenceLedgerPolicy, PresenceOperationLedger, PresenceOperationRecord};
use crate::connection::{ConnectionKey, DisconnectConnectionCommand};

/// Состояние локального хранилища каналов.
pub struct MemoryStoreState {
  /// Ограничения журналов Presence-операций.
  ledger_policy: PresenceLedgerPolicy,

  /// Состояние каналов.
  channels: HashMap<ChannelKey, ChannelState>,

  /// Каналы, к которым присоединено каждое соединение.
  connection_channels: HashMap<ConnectionKey, HashSet<ChannelKey>>,

  /// Журналы обработанных Presence-команд каждого соединения.
  ///
  /// Журнал закрывается при disconnect и удаляется `sweep_presence_ledgers`
  /// после истечения retention.
  presence_ledgers: HashMap<ConnectionKey, PresenceOperationLedger>,
}

impl Default for MemoryStoreState {
  fn default() -> Self {
    Self::new(PresenceLedgerPolicy::default())
  }
}

impl MemoryStoreState {
  pub fn new(ledger_policy: PresenceLedgerPolicy) -> Self {
    Self {
      ledger_policy,
      channels: HashMap::new(),
      connection_channels: HashMap::new(),
      presence_ledgers: HashMap::new(),
    }
  }

  /// Возвращает snapshot канала, не изменяя состояние хранилища.
  pub fn channel_snapshot(&self, channel: &ChannelKey) -> PresenceSnapshot {
    self
      .channels
      .get(channel)
      .map(ChannelState::snapshot)
      .unwrap_or_else(|| ChannelState::default().snapshot())
  }

  /// Сохраняет индивидуальный attachment и возвращает снимок канала.
  ///
  /// Повторный attach того же соединения идемпотентен: attachment
  /// перезаписывается, счётчики не растут, возвращается свежий снимок.
  pub fn attach(
    &mut self,
    command: AttachCommand,
  ) -> Result<ChannelAttachOutcome, ChannelStateStoreError> {
    if command.accounting != AttachmentTracking::Individual {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "individual attachment accounting is required".to_owned(),
      });
    }

    if !command.channel.belongs_to_application(&command.actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let connection_key = ConnectionKey::from(&command.actor);

    // Соединение с закрытым журналом авторитетно завершено: его идентификатор
    // не может начать новый жизненный цикл, пока журнал не очищен.
    if self
      .presence_ledgers
      .get(&connection_key)
      .is_some_and(PresenceOperationLedger::is_closed)
    {
      return Err(ChannelStateStoreError::Conflict {
        message: format!(
          "connection {} is closed and cannot attach",
          command.actor.connection_id.as_str(),
        ),
      });
    }

    let channel = command.channel.clone();

    let attachment = command.to_attachment();

    let channel_state = self
      .channels
      .entry(channel.clone())
      .or_default();

    let occupancy_change = channel_state.save_attachment(attachment.clone())?;

    let snapshot = channel_state.snapshot();

    let event = occupancy_change.map(|occupancy| {
      CommittedPresenceEvent::new(
        Uuid::new_v4(),
        PresenceChannelChanged {
          channel: channel.clone(),
          origin: command.actor.node_instance,
          presence_revision: None,
          occupancy_version: snapshot.occupancy_version,
          member_changes: Vec::new(),
          occupancy: Some(occupancy),
          occurred_at: command.request_time,
        },
      )
    });

    let transition = match event {
      Some(event) => CommittedChannelTransition::Changed(event),
      None => CommittedChannelTransition::Unchanged {
        occupancy_version: snapshot.occupancy_version,
      },
    };

    self
      .connection_channels
      .entry(connection_key)
      .or_default()
      .insert(channel);

    Ok(ChannelAttachOutcome {
      attachment,
      snapshot,
      transition,
      occupancy_shard_baseline: None,
    })
  }

  /// Завершает работу соединения с одним каналом.
  ///
  /// Вместе с attachment удаляет всех Presence-участников этого соединения.
  /// Повторный вызов для уже удалённого attachment считается успешным.
  pub fn detach(&mut self, command: DetachCommand) -> Result<CommittedChannelTransition, ChannelStateStoreError> {
    if !command.channel.belongs_to_application(&command.actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let connection_key = ConnectionKey::from(&command.actor);
    let outcome = match self.channels.get_mut(&command.channel) {
      Some(channel_state) => channel_state.detach_individual(&command.actor)?,
      None => IndividualDetachOutcome::NotAttached {
        occupancy_version: 0,
      },
    };

    self.remove_connection_channel(&connection_key, &command.channel);

    Ok(outcome.into_transition(command))
  }

  /// Удаляет соединение из всех его каналов и очищает журнал Presence.
  ///
  /// Все каналы проверяются до первого изменения состояния.
  /// Повторное отключение возвращает пустой список переходов.
  pub fn disconnect(
    &mut self,
    command: DisconnectConnectionCommand,
  ) -> Result<Vec<CommittedChannelTransition>, ChannelStateStoreError> {
    let connection_key = ConnectionKey::from(&command.actor);

    // Копируем ключи: detach будет изменять обратный индекс.
    let mut channels = self
      .connection_channels
      .get(&connection_key)
      .map(|channels| channels.iter().cloned().collect::<Vec<_>>())
      .unwrap_or_default();

    channels.sort_by(|left, right| left.channel.cmp(&right.channel));

    // Проверяем все каналы до удаления первого attachment.
    for channel in &channels {
      if !channel.belongs_to_application(&command.actor.application_id) {
        return Err(ChannelStateStoreError::InvalidRequest {
          message: "channel and connection belong to different applications"
            .to_owned(),
        });
      }

      if let Some(state) = self.channels.get(channel) {
        state.check_individual_detach(&command.actor)?;
      }
    }

    let mut transitions = Vec::with_capacity(channels.len());

    for channel in channels {
      let transition = self.detach(DetachCommand {
        channel,
        actor: command.actor.clone(),
        request_time: command.request_time,
      })?;

      transitions.push(transition);
    }

    self.connection_channels.remove(&connection_key);

    // Журнал не удаляется: поздний повтор известной операции должен получить
    // прежний результат, а неизвестной — отказ, а не стать новой операцией.
    self
      .ledger_entry(connection_key)
      .close(command.request_time);

    // Закрытые журналы появляются только здесь, поэтому попутная очистка
    // ограничивает их число отключениями за окно retention без фонового таймера.
    self.sweep_presence_ledgers(command.request_time);

    Ok(transitions)
  }

  fn ledger_entry(&mut self, connection_key: ConnectionKey) -> &mut PresenceOperationLedger {
    let capacity = self.ledger_policy.capacity;

    self
      .presence_ledgers
      .entry(connection_key)
      .or_insert_with(|| PresenceOperationLedger::new(capacity))
  }

  /// Применяет клиентскую Presence-команду с дедупликацией по
  /// `(application_id, connection_id, msg_serial)`.
  ///
  /// Поиск в журнале, выполнение и запись результата происходят в одной
  /// критической секции. В журнал попадают оба доменных исхода — `Committed`
  /// и `Rejected`; инфраструктурная ошибка не записывается, и повтор команды
  /// выполняется заново. Отказы о состоянии журнала (`ConflictingReplay`,
  /// `ConnectionClosed`) не записываются, чтобы не затирать исходную запись.
  pub fn apply_presence(
    &mut self,
    command: PresenceBatchCommand,
  ) -> Result<PresenceMutationReceipt, ChannelStateStoreError> {
    let actor = &command.actor.connection_actor;

    if !command.channel.belongs_to_application(&actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let connection_key = ConnectionKey::from(actor);

    if let Some(ledger) = self.presence_ledgers.get(&connection_key) {
      match ledger.lookup(command.msg_serial, &command.request_fingerprint) {
        LedgerLookup::Replay(outcome) => {
          return Ok(PresenceMutationReceipt::replayed(outcome.clone()));
        }
        LedgerLookup::Conflict => {
          return Ok(PresenceMutationReceipt::fresh(PresenceMutationOutcome::Rejected(
            PresenceRejection::ConflictingReplay,
          )));
        }
        LedgerLookup::Evicted => {
          return Ok(PresenceMutationReceipt::fresh(PresenceMutationOutcome::Rejected(
            PresenceRejection::StaleOperation,
          )));
        }
        LedgerLookup::Closed => {
          return Ok(PresenceMutationReceipt::fresh(PresenceMutationOutcome::Rejected(
            PresenceRejection::ConnectionClosed,
          )));
        }
        LedgerLookup::Miss => {}
      }
    }

    let outcome = match self.channels.get_mut(&command.channel) {
      Some(channel_state) => channel_state.apply_presence_batch(&command)?,
      None => PresenceMutationOutcome::Rejected(PresenceRejection::NotAttached),
    };

    self.ledger_entry(connection_key).record(
      command.msg_serial,
      PresenceOperationRecord::new(command.request_fingerprint, outcome.clone()),
    );

    Ok(PresenceMutationReceipt::fresh(outcome))
  }

  /// Удаляет журналы соединений, закрытые раньше `now - retention`.
  ///
  /// Вызывается при каждом disconnect и может вызываться по таймеру.
  /// Возвращает число удалённых журналов. Retention должен быть не меньше
  /// максимального окна retry/resume: после удаления журнала store уже не
  /// отличает поздний повтор от новой операции.
  pub fn sweep_presence_ledgers(&mut self, now: Timestamp) -> usize {
    let before = self.presence_ledgers.len();
    let retention = self.ledger_policy.retention;

    self
      .presence_ledgers
      .retain(|_, ledger| !ledger.is_expired(now, retention));

    before - self.presence_ledgers.len()
  }

  /// Удаляет связь соединения с каналом из обратного индекса.
  fn remove_connection_channel(&mut self, connection: &ConnectionKey, channel: &ChannelKey) {
    let remove_connection = self
      .connection_channels
      .get_mut(connection)
      .is_some_and(|channels| {
        channels.remove(channel);
        channels.is_empty()
      });

    if remove_connection {
      self.connection_channels.remove(connection);
    }
  }

}
