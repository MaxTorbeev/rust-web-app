use support::timestamp::Timestamp;
use crate::connection::ConnectionActor;

/// Команда очистки состояния Presence после завершения соединения.
#[derive(Debug, Clone)]
pub struct DisconnectConnectionCommand {
  /// Контекст соединения: приложение, идентификатор и экземпляр ноды.
  pub actor: ConnectionActor,

  /// Время начала обработки отключения сервером.
  pub request_time: Timestamp,
}