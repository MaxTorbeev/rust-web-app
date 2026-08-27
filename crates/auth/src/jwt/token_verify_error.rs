#[derive(Debug)]
pub enum TokenVerifyError {
  /// Неправильный формат, подпись или алгоритм
  InvalidToken(jsonwebtoken::errors::Error),
  /// Истекший токен
  Expired,
  /// В JWT header отсутствует kid
  MissingKeyId,
  /// Токен подписан неизвестным ключом
  UnexpectedKeyId { expected: String, actual: String },
  /// ClientId указан, но пустой строкой
  EmptyClientId,
  /// iat находится недопустимо в долеком будущем
  IssuedAtInFuture { issued_at: u64, now: u64 },
  /// Строка не содержит корректный json
  InvalidCapability(serde_json::Error),
}

impl From<serde_json::Error> for TokenVerifyError {
  fn from(error: serde_json::Error) -> Self {
    Self::InvalidCapability(error)
  }
}
