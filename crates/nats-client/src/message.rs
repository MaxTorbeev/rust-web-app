use std::time::Duration;

use async_nats::jetstream::{AckKind, Message as DriverMessage};
use bytes::Bytes;

use crate::{AckError, MessageMetadataError};

/// One message delivered by a JetStream consumer.
///
/// Одно сообщение, доставленное consumer-у JetStream. Предоставляет payload,
/// subject, номер попытки и операции управления дальнейшей доставкой.
pub struct NatsMessage {
    inner: DriverMessage,
}

impl NatsMessage {
    pub(crate) fn from_driver(inner: DriverMessage) -> Self {
        Self { inner }
    }

    /// Returns the NATS subject used to route this message.
    ///
    /// Возвращает NATS subject, по которому было маршрутизировано сообщение.
    pub fn subject(&self) -> &str {
        self.inner.message.subject.as_str()
    }

    /// Returns the raw message payload.
    ///
    /// Возвращает исходный payload сообщения без декодирования.
    pub fn payload(&self) -> &Bytes {
        &self.inner.message.payload
    }

    /// Returns how many times JetStream has delivered this stream message.
    ///
    /// Возвращает номер текущей попытки доставки сообщения.
    pub fn delivery_attempt(&self) -> Result<u64, MessageMetadataError> {
        let delivered = self
            .inner
            .info()
            .map_err(MessageMetadataError::parse)?
            .delivered;

        if delivered <= 0 {
            return Err(MessageMetadataError::invalid_attempt(delivered));
        }

        Ok(delivered as u64)
    }

    /// Confirms successful processing and waits for the server confirmation.
    ///
    /// Подтверждает успешную обработку и ждёт подтверждения ACK от сервера.
    pub async fn ack(&self) -> Result<(), AckError> {
        self.inner.double_ack().await.map_err(AckError::from_driver)
    }

    /// Requests redelivery, optionally after the supplied delay.
    ///
    /// Сообщает о временной невозможности обработки и запрашивает повторную
    /// доставку сразу или после указанной задержки.
    pub async fn nak(&self, delay: Option<Duration>) -> Result<(), AckError> {
        self.inner
            .double_ack_with(AckKind::Nak(delay))
            .await
            .map_err(AckError::from_driver)
    }

    /// Stops redelivery of a message that cannot be processed successfully.
    ///
    /// Останавливает повторную доставку сообщения, которое невозможно успешно
    /// обработать.
    pub async fn term(&self) -> Result<(), AckError> {
        self.inner
            .double_ack_with(AckKind::Term)
            .await
            .map_err(AckError::from_driver)
    }

    /// Extends the current `ack_wait` while processing is still active.
    ///
    /// Сообщает JetStream, что обработка продолжается, и продлевает `ack_wait`.
    pub async fn in_progress(&self) -> Result<(), AckError> {
        self.inner
            .ack_with(AckKind::Progress)
            .await
            .map_err(AckError::from_driver)
    }
}
