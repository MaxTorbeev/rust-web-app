use async_nats::jetstream::message::PublishMessage as DriverPublishMessage;
use bytes::Bytes;

use crate::PublishMessageError;

/// An outbound JetStream message independent of the NATS driver API.
#[derive(Clone, Debug)]
pub struct PublishMessage {
    subject: String,
    payload: Bytes,
    message_id: Option<String>,
}

impl PublishMessage {
    pub fn new(
        subject: impl Into<String>,
        payload: Bytes,
    ) -> Result<Self, PublishMessageError> {
        let subject = subject.into();

        if !is_valid_publish_subject(&subject) {
            return Err(PublishMessageError::InvalidSubject);
        }

        Ok(Self {
            subject,
            payload,
            message_id: None,
        })
    }

    /// Sets `Nats-Msg-Id`, which JetStream uses for stream deduplication.
    ///
    /// Retries of the same logical publication must reuse the same identifier.
    pub fn message_id(
        mut self,
        message_id: impl Into<String>,
    ) -> Result<Self, PublishMessageError> {
        let message_id = message_id.into();

        if message_id.is_empty() {
            return Err(PublishMessageError::EmptyMessageId);
        }

        if message_id.contains('\r') || message_id.contains('\n') {
            return Err(PublishMessageError::InvalidMessageId);
        }

        self.message_id = Some(message_id);

        Ok(self)
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    pub(crate) fn into_driver(self) -> (String, DriverPublishMessage) {
        let mut message = DriverPublishMessage::build().payload(self.payload);

        if let Some(message_id) = self.message_id {
            message = message.message_id(message_id);
        }

        (self.subject, message)
    }
}

fn is_valid_publish_subject(subject: &str) -> bool {
    !subject.is_empty()
        && !subject
            .bytes()
            .any(|character| matches!(character, b' ' | b'\t' | b'\r' | b'\n'))
}

#[cfg(test)]
mod tests {
    use async_nats::header::NATS_MESSAGE_ID;

    use super::*;

    #[test]
    fn prepares_subject_and_payload_without_headers() {
        let message = PublishMessage::new("events.messages", Bytes::from_static(b"payload"))
            .expect("publish message must be valid");

        assert_eq!(message.subject(), "events.messages");
        assert_eq!(message.payload(), &Bytes::from_static(b"payload"));

        let (subject, message) = message.into_driver();
        let message = message.outbound_message(subject);

        assert_eq!(message.subject.as_str(), "events.messages");
        assert_eq!(message.payload, Bytes::from_static(b"payload"));
        assert!(message.headers.is_none());
    }

    #[test]
    fn sets_nats_message_id_header() {
        let (subject, message) =
            PublishMessage::new("events.messages", Bytes::from_static(b"payload"))
                .expect("publish message must be valid")
                .message_id("event-123")
                .expect("message ID must be valid")
                .into_driver();
        let message = message.outbound_message(subject);
        let headers = message.headers.expect("message headers must be present");

        assert_eq!(headers.len(), 1);
        assert_eq!(
            headers
                .get(NATS_MESSAGE_ID)
                .expect("Nats-Msg-Id header must be present")
                .as_str(),
            "event-123",
        );
    }

    #[test]
    fn rejects_invalid_publish_subjects() {
        for subject in ["", "events messages", "events\tmessages", "events\r\nmessages"] {
            let result = PublishMessage::new(subject, Bytes::new());

            assert_eq!(result.unwrap_err(), PublishMessageError::InvalidSubject);
        }
    }

    #[test]
    fn rejects_invalid_message_ids_without_panicking() {
        let message = PublishMessage::new("events.messages", Bytes::new())
            .expect("publish message must be valid");

        assert_eq!(
            message.clone().message_id("").unwrap_err(),
            PublishMessageError::EmptyMessageId,
        );

        for message_id in ["event-123\r", "event-123\n", "event-123\r\nInjected: true"] {
            assert_eq!(
                message.clone().message_id(message_id).unwrap_err(),
                PublishMessageError::InvalidMessageId,
            );
        }
    }

    #[test]
    fn cloned_message_preserves_deduplication_identity_for_retry() {
        let message = PublishMessage::new(
            "events.messages",
            Bytes::from_static(b"payload"),
        )
        .expect("publish message must be valid")
        .message_id("event-123")
        .expect("message ID must be valid");

        let (first_subject, first) = message.clone().into_driver();
        let (retry_subject, retry) = message.into_driver();

        assert_eq!(first_subject, retry_subject);

        let first = first.outbound_message(first_subject);
        let retry = retry.outbound_message(retry_subject);

        assert_eq!(first.payload, retry.payload);
        assert_eq!(first.headers, retry.headers);
    }

    #[test]
    fn replacing_message_id_keeps_one_header() {
        let (subject, message) = PublishMessage::new("events.messages", Bytes::new())
            .expect("publish message must be valid")
            .message_id("event-123")
            .expect("message ID must be valid")
            .message_id("event-456")
            .expect("replacement message ID must be valid")
            .into_driver();
        let message = message.outbound_message(subject);
        let headers = message.headers.expect("message headers must be present");

        assert_eq!(headers.len(), 1);
        assert_eq!(
            headers
                .get(NATS_MESSAGE_ID)
                .expect("Nats-Msg-Id header must be present")
                .as_str(),
            "event-456",
        );
    }
}
