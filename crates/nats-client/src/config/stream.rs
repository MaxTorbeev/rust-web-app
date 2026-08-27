use std::time::Duration;

use async_nats::jetstream::stream::{Config as DriverStreamConfig, RetentionPolicy, StorageType};

use crate::error::{StreamConfigError, StreamLimitsError};
use crate::validation::{is_valid_entity_name, is_valid_subject_filter};

/// Bounded retention settings for a JetStream stream.
///
/// Ограничения хранения потока JetStream: число сообщений, размер одного
/// сообщения, общий объём и максимальное время хранения.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamLimits {
    max_messages: i64,
    max_message_size: i32,
    max_bytes: i64,
    max_age: Duration,
}

impl StreamLimits {
    /// Builds positive, bounded stream retention limits.
    ///
    /// Создаёт ограничения хранения и проверяет, что каждый лимит больше нуля.
    pub fn try_new(
        max_messages: i64,
        max_message_size: i32,
        max_bytes: i64,
        max_age: Duration,
    ) -> Result<Self, StreamLimitsError> {
        if max_messages <= 0 {
            return Err(StreamLimitsError::InvalidMaxMessages { max_messages });
        }
        if max_message_size <= 0 {
            return Err(StreamLimitsError::InvalidMaxMessageSize { max_message_size });
        }
        if max_bytes <= 0 {
            return Err(StreamLimitsError::InvalidMaxBytes { max_bytes });
        }
        if max_age.is_zero() {
            return Err(StreamLimitsError::ZeroMaxAge);
        }

        Ok(Self {
            max_messages,
            max_message_size,
            max_bytes,
            max_age,
        })
    }
}

/// Authoritative configuration of one limits-retention JetStream stream.
///
/// Ожидаемая конфигурация одного JetStream-потока с политикой `Limits` и
/// файловым хранением. Используется как контракт при создании и проверке потока.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamConfig {
    name: String,
    subjects: Vec<String>,
    limits: StreamLimits,
    replicas: usize,
    duplicate_window: Duration,
}

impl StreamConfig {
    /// Builds and validates a stream configuration.
    ///
    /// Создаёт конфигурацию потока и проверяет имя, subjects, число реплик,
    /// лимиты хранения и окно дедупликации `Nats-Msg-Id`.
    pub fn try_new<I, S>(
        name: impl Into<String>,
        subjects: I,
        limits: StreamLimits,
        replicas: usize,
        duplicate_window: Duration,
    ) -> Result<Self, StreamConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let name = name.into();
        let subjects: Vec<String> = subjects.into_iter().map(Into::into).collect();

        if !is_valid_entity_name(&name) {
            return Err(StreamConfigError::InvalidName { value: name });
        }

        if subjects.is_empty() {
            return Err(StreamConfigError::NoSubjects);
        }

        for subject in &subjects {
            if !is_valid_subject_filter(subject) {
                return Err(StreamConfigError::InvalidSubject {
                    value: subject.clone(),
                });
            }
        }

        let mut unique_subjects = subjects.clone();
        unique_subjects.sort_unstable();
        unique_subjects.dedup();
        if unique_subjects.len() != subjects.len() {
            return Err(StreamConfigError::DuplicateSubjects);
        }

        if !(1..=5).contains(&replicas) {
            return Err(StreamConfigError::InvalidReplicas { replicas });
        }

        if duplicate_window.is_zero() {
            return Err(StreamConfigError::ZeroDuplicateWindow);
        }

        Ok(Self {
            name,
            subjects,
            limits,
            replicas,
            duplicate_window,
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn to_driver_config(&self) -> DriverStreamConfig {
        DriverStreamConfig {
            name: self.name.clone(),
            subjects: self.subjects.clone(),
            retention: RetentionPolicy::Limits,
            max_messages: self.limits.max_messages,
            max_message_size: self.limits.max_message_size,
            max_bytes: self.limits.max_bytes,
            max_age: self.limits.max_age,
            storage: StorageType::File,
            num_replicas: self.replicas,
            duplicate_window: self.duplicate_window,
            ..Default::default()
        }
    }

    pub(crate) fn incompatible_fields(&self, actual: &DriverStreamConfig) -> Vec<&'static str> {
        let expected = self.to_driver_config();
        let mut fields = Vec::new();

        if actual.name != expected.name {
            fields.push("name");
        }
        if !same_subjects(&actual.subjects, &expected.subjects) {
            fields.push("subjects");
        }
        if actual.retention != expected.retention {
            fields.push("retention");
        }
        if actual.max_messages != expected.max_messages {
            fields.push("max_messages");
        }
        if actual.max_message_size != expected.max_message_size {
            fields.push("max_message_size");
        }
        if actual.max_bytes != expected.max_bytes {
            fields.push("max_bytes");
        }
        if actual.max_age != expected.max_age {
            fields.push("max_age");
        }
        if actual.storage != expected.storage {
            fields.push("storage");
        }
        if actual.num_replicas != expected.num_replicas {
            fields.push("replicas");
        }
        if actual.duplicate_window != expected.duplicate_window {
            fields.push("duplicate_window");
        }

        fields
    }
}

fn same_subjects(left: &[String], right: &[String]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort_unstable();
    right.sort_unstable();

    left == right
}
