use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::{DispatchError, Event, EventMessage, HandlerError, HandlerRegistrationError};

type EventHandlerFuture = Pin<Box<dyn Future<Output = Result<(), HandlerError>> + Send + 'static>>;

type EventHandler =
  Box<dyn Fn(&EventMessage) -> Result<EventHandlerFuture, DispatchError> + Send + Sync + 'static>;

/// Хранит обязательные обработчики и применяет входящие envelope локально.
///
/// Dispatcher ничего не публикует и ничего не знает о NATS, ACK, повторных
/// попытках или дедупликации. Его задача ограничена четырьмя действиями:
///
/// 1. найти handler по стабильному [`Event::NAME`];
/// 2. проверить имя и версию envelope;
/// 3. декодировать payload в конкретный Rust-тип события;
/// 4. дождаться завершения обязательного handler-а.
///
/// После регистрации всех handler-ов dispatcher обычно помещается в `Arc` и
/// больше не изменяется.
#[derive(Default)]
pub struct EventDispatcher {
  handlers: HashMap<&'static str, EventHandler>,
}

impl EventDispatcher {
  /// Создаёт пустой dispatcher без зарегистрированных handler-ов.
  ///
  /// После создания приложение должно зарегистрировать все обязательные
  /// handler-ы и только затем передать dispatcher в `Arc`.
  pub fn new() -> Self {
    Self::default()
  }

  /// Регистрирует единственный обязательный handler для типа события `E`.
  ///
  /// Тип события выводится из аргумента closure. Например, в записи
  /// `|event: UserCreated|` параметр `E` равен `UserCreated`, поэтому dispatcher
  /// может прочитать `UserCreated::NAME` и `UserCreated::VERSION`.
  ///
  /// Handler возвращает [`HandlerError`], если не смог применить событие. Он
  /// обязан сам указать, является ли причина временной или постоянной.
  ///
  /// Все handler-ы нужно зарегистрировать до передачи dispatcher-а между
  /// задачами через `Arc`.
  pub fn register<E, F, Fut>(&mut self, handler: F) -> Result<(), HandlerRegistrationError>
  where
    E: Event,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
  {
    if self.handlers.contains_key(E::NAME) {
      return Err(HandlerRegistrationError::AlreadyRegistered {
        event_name: E::NAME.to_owned(),
      });
    }

    self.handlers.insert(
      E::NAME,
      Box::new(move |message| {
        let event = message.decode_event::<E>()?;
        let future: EventHandlerFuture = Box::pin(handler(event));

        Ok(future)
      }),
    );

    Ok(())
  }

  /// Применяет уже полученный envelope к локальному обязательному handler-у.
  ///
  /// `Ok(())` означает, что payload успешно декодирован, handler найден и его
  /// future завершилась успешно. Метод не означает доставку клиенту по сети и
  /// не выполняет дедупликацию.
  ///
  /// При ошибке возвращается [`DispatchError`] с доступным классом
  /// `Retryable` или `Permanent`. Сам dispatcher не выполняет retry — это
  /// решение будущего входящего processor-а и конкретного транспорта.
  pub async fn dispatch(&self, message: &EventMessage) -> Result<(), DispatchError> {
    let handler = self.handlers.get(message.event_name()).ok_or_else(|| {
      DispatchError::HandlerNotRegistered {
        event_name: message.event_name().to_owned(),
      }
    })?;

    handler(message)?
      .await
      .map_err(|source| DispatchError::Handler {
        event_name: message.event_name().to_owned(),
        source,
      })
  }
}
