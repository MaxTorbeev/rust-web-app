use crate::error::NatsConfigError;

/// Connection endpoints for a NATS server or cluster.
///
/// Список адресов серверов NATS, используемый для подключения к одиночному
/// серверу или кластеру.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NatsConfig {
    servers: Vec<String>,
}

impl NatsConfig {
    /// Builds and validates a non-empty server list.
    ///
    /// Создаёт конфигурацию и проверяет, что список серверов не пуст, а адреса
    /// не содержат пробельных символов.
    pub fn try_new<I, S>(servers: I) -> Result<Self, NatsConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers: Vec<String> = servers.into_iter().map(Into::into).collect();

        if servers.is_empty() {
            return Err(NatsConfigError::NoServers);
        }

        if let Some((index, server)) = servers
            .iter()
            .enumerate()
            .find(|(_, server)| server.is_empty() || server.chars().any(char::is_whitespace))
        {
            return Err(NatsConfigError::InvalidServer {
                index,
                value: server.clone(),
            });
        }

        Ok(Self { servers })
    }

    /// Returns the configured server addresses.
    ///
    /// Возвращает настроенные адреса серверов NATS.
    pub fn servers(&self) -> &[String] {
        &self.servers
    }
}
