use std::sync::Arc;
use std::time::Duration;

use crate::{
  DedupClaim, DedupKey, DedupStore, EventDispatcher, EventMessage, IncomingEventError,
  IncomingEventOutcome, IncomingEventProcessorConfigError,
};

/// Защищает входящие события от повторной обработки и применяет их локально.
///
/// Перед вызовом [`EventDispatcher`] processor пытается получить в
/// [`DedupStore`] исключительное право на обработку ключа
/// `(scope, event_id)`. Поэтому один и тот же `event_id` может независимо
/// обрабатываться в разных областях дедупликации.
///
/// Processor не знает, откуда пришло событие, и не управляет подтверждениями
/// транспорта. Он только возвращает результат обработки. JetStream, Kafka или
/// другой consumer самостоятельно преобразует этот результат в ACK, NAK,
/// повторную доставку либо окончательное отклонение сообщения.
pub struct IncomingEventProcessor {
  dispatcher: Arc<EventDispatcher>,
  dedup_store: Arc<dyn DedupStore>,
  scope: String,
  lease_ttl: Duration,
  /// Сколько времени после успешной обработки `event_id` считается завершённым.
  completed_record_ttl: Duration,
}

impl IncomingEventProcessor {
  /// Создаёт processor с явно заданной областью дедупликации и сроками жизни
  /// записей.
  ///
  /// Область дедупликации (`scope`) — это идентификатор независимого
  /// получателя или группы обработчиков. Вместе с `event_id` она образует ключ
  /// `(scope, event_id)`, по которому определяется, обрабатывалось ли событие.
  /// Это не NATS subject и не префикс Redis-ключей.
  ///
  /// Выбор `scope` зависит от способа доставки:
  ///
  /// - для `AllNodes` каждая нода использует собственный стабильный
  ///   идентификатор. Благодаря этому одно событие будет применено по одному
  ///   разу на каждой ноде;
  /// - для `WorkQueue` все consumers одной рабочей группы используют общий
  ///   идентификатор. Благодаря этому событие применит только один участник
  ///   группы.
  ///
  /// `scope` должен сохраняться после перезапуска процесса. Если генерировать
  /// его при каждом запуске, ранее обработанные события перестанут
  /// распознаваться как дубликаты.
  ///
  /// `lease_ttl` ограничивает время временного права на обработку события. Если
  /// consumer аварийно завершится, после истечения этого срока другой consumer
  /// сможет продолжить обработку.
  ///
  /// `completed_record_ttl` определяет, как долго после успешного выполнения
  /// handler-а событие распознаётся как уже обработанное.
  ///
  /// Возвращает ошибку, если `scope` пустой либо один из сроков равен нулю.
  pub fn try_new(
    dispatcher: Arc<EventDispatcher>,
    dedup_store: Arc<dyn DedupStore>,
    scope: impl Into<String>,
    lease_ttl: Duration,
    completed_record_ttl: Duration,
  ) -> Result<Self, IncomingEventProcessorConfigError> {
    let scope = scope.into();

    if scope.is_empty() {
      return Err(IncomingEventProcessorConfigError::EmptyScope);
    }

    if lease_ttl.is_zero() {
      return Err(IncomingEventProcessorConfigError::ZeroLeaseTtl);
    }

    if completed_record_ttl.is_zero() {
      return Err(IncomingEventProcessorConfigError::ZeroCompletedRecordTtl);
    }

    Ok(Self {
      dispatcher,
      dedup_store,
      scope,
      lease_ttl,
      completed_record_ttl,
    })
  }

  /// Проверяет состояние дедупликации и при необходимости запускает handler.
  ///
  /// Возможные результаты:
  ///
  /// - [`IncomingEventOutcome::Applied`] — право на обработку получено,
  ///   handler успешно выполнен, а событие отмечено завершённым;
  /// - [`IncomingEventOutcome::Duplicate`] — событие уже было успешно
  ///   обработано, поэтому handler повторно не запускается;
  /// - [`IncomingEventOutcome::InProgress`] — событие сейчас обрабатывает
  ///   другой consumer. Handler не запускается, а вызывающая сторона должна
  ///   повторить проверку не раньше указанного `retry_after`.
  ///
  /// Повторная проверка после `InProgress` нужна даже тогда, когда другой
  /// consumer работает нормально: она подтвердит завершение события и вернёт
  /// `Duplicate`. Если же тот consumer аварийно завершился, его временное право
  /// истечёт и событие сможет безопасно забрать другой исполнитель.
  ///
  /// Если handler завершается с ошибкой, processor пытается освободить
  /// временное право на обработку. Решение о повторной доставке принимает
  /// вызывающий transport consumer на основании [`IncomingEventError`].
  pub async fn process(
    &self,
    message: &EventMessage,
  ) -> Result<IncomingEventOutcome, IncomingEventError> {
    let key = DedupKey::new(self.scope.clone(), message.event_id());

    let claim = self
      .dedup_store
      .claim(&key, self.lease_ttl)
      .await
      .map_err(|source| IncomingEventError::Claim { source })?;

    match claim {
      DedupClaim::Completed => Ok(IncomingEventOutcome::Duplicate),

      DedupClaim::InProgress { retry_after } => {
        Ok(IncomingEventOutcome::InProgress { retry_after })
      }

      DedupClaim::Acquired(lease) => {
        if let Err(source) = self.dispatcher.dispatch(message).await {
          let release_error = self.dedup_store.release(&lease).await.err();

          return Err(IncomingEventError::Dispatch {
            source,
            release_error,
          });
        }

        self
          .dedup_store
          .complete(&lease, self.completed_record_ttl)
          .await
          .map_err(|source| IncomingEventError::Complete { source })?;

        Ok(IncomingEventOutcome::Applied)
      }
    }
  }
}
