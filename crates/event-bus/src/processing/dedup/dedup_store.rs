use crate::{DedupClaim, DedupKey, DedupLease, DedupStoreError};
use std::pin::Pin;
use std::time::Duration;

pub type DedupStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, DedupStoreError>> + Send + 'a>>;

pub trait DedupStore: Send + Sync {
  /// Попытка атомарно получить исключительное право на обработку события.
  ///
  /// Возвращает:
  ///
  /// - [`DedupClaim::Acquired`], если событие можно обрабатывать;
  /// - [`DedupClaim::Completed`], если событие уже успешно обработано
  ///   и отметка ещё не истекла;
  /// - [`DedupClaim::InProgress`], если действующим lease владеет другой worker.
  ///
  /// Реализация обязана гарантировать, что для одного [`DedupKey`]
  /// одновременно существует не более одного действующего lease.
  fn claim<'a>(
    &'a self,
    key: &'a DedupKey,
    lease_ttl: Duration,
  ) -> DedupStoreFuture<'a, DedupClaim>;

  /// Атомарно отметить событие как успешно обработанное.
  ///
  /// Метод следует вызывать только после успешного завершения обязательного
  /// handler-а. После успеха последующие вызовы [`DedupStore::claim`] должны
  /// возвращать [`DedupClaim::Completed`] в течение `completed_ttl`.
  ///
  /// Метод отмечает событие завершённым только тогда, когда `token` из переданного
  /// `lease` совпадает с `token`, сохранённым в хранилище. Это не позволяет одному
  /// исполнителю завершить обработку, начатую другим.
  fn complete<'a>(
    &'a self,
    lease: &'a DedupLease,
    completed_ttl: Duration,
  ) -> DedupStoreFuture<'a, ()>;

  /// Освобождает lease после неуспешной попытки обработки.
  ///
  /// После освобождения другой worker может сразу получить
  /// [`DedupClaim::Acquired`], не ожидая окончания `lease_ttl`.
  ///
  /// Реализация должна удалить lease только при совпадении token. Метод не
  /// должен удалять отметку `Completed` или lease, уже принадлежащий другому
  /// worker-у. Если процесс завершился аварийно и не вызвал `release`, lease
  /// будет освобождён автоматически после истечения TTL.
  fn release<'a>(&'a self, lease: &'a DedupLease) -> DedupStoreFuture<'a, ()>;
}
