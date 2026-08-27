use std::sync::Arc;

use crate::{
  DedupClaim, DedupKey, DedupStore, EventDispatcher, EventMessage, IncomingEventError,
  IncomingEventOutcome, IncomingEventProcessorConfig,
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
  config: IncomingEventProcessorConfig,
}

impl IncomingEventProcessor {
  /// Создаёт processor из заранее проверенной конфигурации.
  pub fn new(
    dispatcher: Arc<EventDispatcher>,
    dedup_store: Arc<dyn DedupStore>,
    config: IncomingEventProcessorConfig,
  ) -> Self {
    Self {
      dispatcher,
      dedup_store,
      config,
    }
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
  ///
  /// Выполнение handler-а ограничено `processing_timeout`. При превышении
  /// лимита его future удаляется, lease освобождается, а вызывающая сторона
  /// получает retryable-ошибку. Ограничение кооперативное: handler должен
  /// регулярно возвращать управление executor-у и не выполнять блокирующую
  /// работу в async-контексте. Побочные эффекты, которые handler уже успел
  /// выполнить, не откатываются, поэтому обработчики всё равно должны быть
  /// идемпотентными.
  ///
  /// # Panics
  ///
  /// После получения lease метод паникует, если future выполняется вне Tokio
  /// runtime с включённым time driver.
  pub async fn process(
    &self,
    message: &EventMessage,
  ) -> Result<IncomingEventOutcome, IncomingEventError> {
    let key = DedupKey::new(self.config.scope(), message.event_id());

    let claim = self
      .dedup_store
      .claim(&key, self.config.lease_ttl())
      .await
      .map_err(|source| IncomingEventError::Claim { source })?;

    match claim {
      DedupClaim::Completed => Ok(IncomingEventOutcome::Duplicate),

      DedupClaim::InProgress { retry_after } => {
        Ok(IncomingEventOutcome::InProgress { retry_after })
      }

      DedupClaim::Acquired(lease) => {
        let dispatch_result = tokio::time::timeout(
          self.config.processing_timeout(),
          self.dispatcher.dispatch(message),
        )
        .await;

        match dispatch_result {
          Ok(Ok(())) => {}
          Ok(Err(source)) => {
            let release_error = self.dedup_store.release(&lease).await.err();

            return Err(IncomingEventError::Dispatch {
              source,
              release_error,
            });
          }
          Err(_) => {
            let release_error = self.dedup_store.release(&lease).await.err();

            return Err(IncomingEventError::ProcessingTimeout {
              timeout: self.config.processing_timeout(),
              release_error,
            });
          }
        }

        self
          .dedup_store
          .complete(&lease, self.config.completed_record_ttl())
          .await
          .map_err(|source| IncomingEventError::Complete { source })?;

        Ok(IncomingEventOutcome::Applied)
      }
    }
  }
}
