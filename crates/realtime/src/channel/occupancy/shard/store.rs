use std::{future::Future, pin::Pin};

use crate::{ChannelStateStoreError, OccupancyShardFlushResult, OccupancyShardSnapshot};

pub type OccupancyShardStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, ChannelStateStoreError>> + Send + 'a>>;

/// Хранилище снимков локальных счётчиков Occupancy.
///
/// Для каждой пары `(node_instance, channel)` хранит последнюю версию
/// абсолютных счётчиков и учитывает её при расчёте общих метрик канала.
pub trait OccupancyShardStore: Send + Sync {
  /// Сохраняет абсолютные счётчики Occupancy одного канала,
  /// собранные конкретным экземпляром ноды.
  ///
  /// Если версия снимка новее сохранённой, хранилище заменяет предыдущие
  /// счётчики этого экземпляра ноды и применяет разницу к общим счётчикам
  /// канала как одну неделимую операцию.
  ///
  /// Повторная отправка той же или более старой версии не изменяет состояние.
  /// Метод не изменяет ревизию Presence и не создаёт отдельное событие Presence.
  ///
  /// Возвращает актуальную версию и общие метрики Occupancy, а также признак
  /// перехода любого общего счётчика через нулевую границу.
  ///
  /// Срок аренды сегмента назначается хранилищем. Redis-реализация использует
  /// серверное время Redis, а не часы экземпляра ноды.
  ///
  /// # Errors
  ///
  /// Возвращает [`ChannelStateStoreError`], если снимок не может быть проверен
  /// или сохранён.
  fn flush(
    &self,
    snapshot: OccupancyShardSnapshot,
  ) -> OccupancyShardStoreFuture<'_, OccupancyShardFlushResult>;
}
