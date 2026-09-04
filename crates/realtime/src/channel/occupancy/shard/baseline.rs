/// Счётчики конкретного запуска ноды, уже включённые в общий Occupancy канала.
///
/// Конкретный запуск определяется парой `node_id` и `boot_generation`.
/// Хранилище возвращает эти значения вместе с общим снимком, чтобы заменить
/// их текущими локальными счётчиками без двойного подсчёта.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OccupancyShardBaseline {
  /// Версия сегмента, сохранённая в хранилище.
  pub version: u64,
  pub connections: u64,
  pub subscribers: u64,
  pub presence_subscribers: u64,
}
