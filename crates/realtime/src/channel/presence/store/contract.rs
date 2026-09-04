use std::{future::Future, pin::Pin};

use crate::channel::presence::command::PresenceBatchCommand;
use crate::channel::presence::snapshot::PresenceSnapshot;
use crate::{
  ChannelKey, ChannelStateStoreError, OccupancyShardFlushResult, OccupancyShardSnapshot,
  PresenceMutationReceipt,
};

pub type PresenceStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, ChannelStateStoreError>> + Send + 'a>>;

pub trait PresenceStore: Send + Sync {
  fn apply_presence(
    &self,
    command: PresenceBatchCommand,
  ) -> PresenceStoreFuture<'_, PresenceMutationReceipt>;

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot>;

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
  /// # Errors
  ///
  /// Возвращает [`ChannelStateStoreError`], если снимок не может быть проверен
  /// или сохранён.
  fn flush_occupancy_shard(
    &self,
    shard: OccupancyShardSnapshot,
  ) -> PresenceStoreFuture<'_, OccupancyShardFlushResult>;
}
