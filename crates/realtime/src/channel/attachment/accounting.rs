/// Способ учёта Attachment в состоянии канала.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachmentAccounting {
  /// Attachment хранится как отдельная запись.
  Exact,

  /// Attachment учитывается в агрегированных счётчиках экземпляра ноды.
  Aggregated,
}