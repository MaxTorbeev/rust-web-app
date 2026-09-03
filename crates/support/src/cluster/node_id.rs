use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

/// Стабильный идентификатор экземпляра приложения внутри кластера.
///
/// Значение задаётся оператором через `APP_NODE_ID`, сохраняется между
/// перезапусками процесса и должно быть уникальным среди одновременно
/// работающих узлов.
///
/// Формат намеренно ограничен: идентификатор начинается с ASCII-буквы или
/// цифры и далее содержит только ASCII-буквы, цифры, `-` и `_`. Благодаря
/// этому его можно без дополнительного экранирования использовать в именах
/// кластерных ресурсов и ключах хранилища.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
  /// Создаёт `NodeId` из переданного значения после проверки его формата.
  ///
  /// Метод не генерирует идентификатор и не изменяет полученное значение.
  ///
  /// # Errors
  ///
  /// Возвращает [`NodeIdError::Empty`], если значение пустое, и
  /// [`NodeIdError::InvalidFormat`], если первый символ не является
  /// ASCII-буквой или цифрой либо остальные символы содержат что-либо кроме
  /// ASCII-букв, цифр, `-` и `_`.
  pub fn try_new(value: impl Into<String>) -> Result<Self, NodeIdError> {
    let value = value.into();
    let mut bytes = value.bytes();

    let Some(first) = bytes.next() else {
      return Err(NodeIdError::Empty);
    };

    if !first.is_ascii_alphanumeric()
      || !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
      return Err(NodeIdError::InvalidFormat { value });
    }

    Ok(Self(value))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl Display for NodeId {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for NodeId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let value = String::deserialize(deserializer)?;

    Self::try_new(value).map_err(D::Error::custom)
  }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodeIdError {
  #[error("node id must not be empty")]
  Empty,

  #[error(
    "invalid node id {value:?}: it must start with an ASCII letter or digit and contain only ASCII letters, digits, '-' or '_'"
  )]
  InvalidFormat { value: String },
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_supported_node_id() {
    let node_id = NodeId::try_new("app-node_1").expect("node id must be valid");

    assert_eq!(node_id.as_str(), "app-node_1");
    assert_eq!(node_id.to_string(), "app-node_1");
  }

  #[test]
  fn rejects_empty_node_id() {
    assert_eq!(NodeId::try_new(""), Err(NodeIdError::Empty));
  }

  #[test]
  fn rejects_unsupported_node_id_format() {
    for value in ["-app-node", "_app-node", "app.node", "app node", "узел"] {
      assert_eq!(
        NodeId::try_new(value),
        Err(NodeIdError::InvalidFormat {
          value: value.to_owned(),
        })
      );
    }
  }

  #[test]
  fn deserialization_preserves_validation() {
    let deserializer =
      serde::de::value::StrDeserializer::<serde::de::value::Error>::new("invalid.node");
    let error = NodeId::deserialize(deserializer)
      .expect_err("deserialization must reject an invalid node id");

    assert!(error.to_string().contains("invalid node id"));
  }
}
