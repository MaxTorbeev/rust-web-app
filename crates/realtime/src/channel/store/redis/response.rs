use std::collections::{BTreeSet, HashMap};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use event_bus::Event;
use redis_client::ScriptValue;
use serde::de::DeserializeOwned;
use support::NodeInstance;
use support::timestamp::Timestamp;
use uuid::Uuid;

use crate::{
  Attachment, ChannelAttachOutcome, ChannelStateStoreError, CommittedChannelTransition,
  CommittedPresenceEvent, OccupancyCategory, OccupancyChange, OccupancyMetrics,
  PresenceChangeAction, PresenceChannelChanged, PresenceMember, PresenceMemberChange,
  PresenceMutationOutcome, PresenceMutationReceipt, PresenceRejection, PresenceSnapshot,
};

use super::member::MemberPayload;
use super::protocol::generation;

/// Разбирает metadata кандидатов и сверяет их identity с generation index.
pub(super) fn decode_expired_generations(
  value: &ScriptValue,
) -> Result<Vec<NodeInstance>, ChannelStateStoreError> {
  let ScriptValue::Array(entries) = value else {
    return Err(invalid("invalid expired_generations Lua reply"));
  };
  entries
    .iter()
    .map(|entry| {
      let ScriptValue::Array(fields) = entry else {
        return Err(invalid("invalid expired generation entry"));
      };
      let [id, metadata] = fields.as_slice() else {
        return Err(invalid("invalid expired generation entry"));
      };
      let instance: NodeInstance = decode_json(metadata, "generation metadata")?;
      if generation(&instance) != decode_text(id, "generation")? {
        return Err(invalid(
          "generation metadata does not match its index entry",
        ));
      }
      Ok(instance)
    })
    .collect()
}

/// Собирает результат attach из snapshot и зафиксированного transition.
pub(super) fn decode_attach(
  value: &ScriptValue,
) -> Result<ChannelAttachOutcome, ChannelStateStoreError> {
  let (snapshot, transition) = decode_attach_reply(value)?;
  Ok(ChannelAttachOutcome {
    snapshot: decode_snapshot(snapshot)?,
    transition: decode_transition(transition)?,
  })
}

/// Разбирает отсутствие изменений или событие, записанное в outbox.
pub(super) fn decode_transition(
  value: &ScriptValue,
) -> Result<CommittedChannelTransition, ChannelStateStoreError> {
  let invalid_transition = || ChannelStateStoreError::Internal {
    message: "invalid transition in Lua reply".to_owned(),
  };
  let ScriptValue::Array(values) = value else {
    return Err(invalid_transition());
  };
  match values.as_slice() {
    [ScriptValue::Bytes(kind), version] if kind == b"unchanged" => {
      Ok(CommittedChannelTransition::Unchanged {
        occupancy_version: decode_u64(version, "occupancy_version")?,
      })
    }
    [ScriptValue::Bytes(kind), outbox] if kind == b"changed" => {
      Ok(CommittedChannelTransition::Changed(decode_outbox(outbox)?))
    }
    [ScriptValue::Integer(0), code, message] => Err(decode_rejection(
      decode_text(code, "code")?,
      decode_text(message, "message")?.to_owned(),
    )),
    _ => Err(invalid_transition()),
  }
}

/// Собирает событие из неизменяемых полей attach.v2, detach.v1 или presence.v2.
/// Один декодер используется для ответа, replay и публикации outbox.
pub(super) fn decode_outbox(
  value: &ScriptValue,
) -> Result<CommittedPresenceEvent, ChannelStateStoreError> {
  let fields = decode_fields(value)?;
  let field = |name: &str| required_field(&fields, name);
  let format = decode_text(field("format")?, "format")?;
  if !matches!(format, "attach.v2" | "detach.v1" | "presence.v2")
    || decode_text(field("event_name")?, "event_name")? != PresenceChannelChanged::NAME
    || decode_u64(field("schema_version")?, "schema_version")?
      != u64::from(PresenceChannelChanged::VERSION)
  {
    return Err(invalid("unsupported outbox format or event schema"));
  }
  let event_id = Uuid::parse_str(decode_text(field("event_id")?, "event_id")?)
    .map_err(|error| invalid(format!("invalid outbox event_id: {error}")))?;
  let occurred_at = Timestamp::from_millis(decode_u64(field("occurred_at_ms")?, "occurred_at_ms")?);
  let revision = field("presence_revision")?;
  let presence_revision = if decode_text(revision, "presence_revision")?.is_empty() {
    None
  } else {
    Some(decode_u64(revision, "presence_revision")?)
  };

  let mut member_changes = Vec::new();
  let origin = if format != "presence.v2" {
    let mut removed = Vec::new();
    for index in 1..=decode_u64(field("removed_count")?, "removed_count")? {
      let name = format!("removed.{index}");
      removed.push(decode_json::<MemberPayload>(field(&name)?, &name)?);
    }
    removed.sort_unstable_by(|left, right| left.client_id.cmp(&right.client_id));
    for (index, member) in removed.into_iter().enumerate() {
      member_changes.push(PresenceMemberChange {
        action: PresenceChangeAction::Leave,
        data: decode_member_data(&member)?,
        connection_id: member.connection_id,
        client_id: member.client_id,
        message_id: format!("server:{event_id}:{index}"),
        timestamp: occurred_at,
      });
    }
    if format == "attach.v2" {
      let attachment: Attachment = decode_json(field("attachment_json")?, "attachment_json")?;
      attachment.node_instance
    } else {
      decode_json(field("node_instance_json")?, "node_instance_json")?
    }
  } else {
    if presence_revision.is_none() {
      return Err(invalid("missing Presence revision in outbox"));
    }
    let count = decode_u64(field("change_count")?, "change_count")?;
    if count == 0 {
      return Err(invalid("empty Presence batch in outbox"));
    }
    for index in 1..=count {
      let prefix = format!("change.{index}.");
      let action = match decode_text(field(&format!("{prefix}action"))?, "action")? {
        "enter" => PresenceChangeAction::Enter,
        "update" => PresenceChangeAction::Update,
        "leave" => PresenceChangeAction::Leave,
        _ => return Err(invalid("invalid Presence action in outbox")),
      };
      let member: MemberPayload = decode_json(field(&format!("{prefix}member"))?, "member")?;
      let data = match fields.get(format!("{prefix}previous").as_str()) {
        Some(previous) => {
          let previous: MemberPayload = decode_json(previous, "previous")?;
          decode_member_data(&previous)?
        }
        None => decode_member_data(&member)?,
      };
      member_changes.push(PresenceMemberChange {
        action,
        data,
        connection_id: member.connection_id,
        client_id: member.client_id,
        message_id: member.last_message_id,
        timestamp: occurred_at,
      });
    }
    decode_json(field("node_instance_json")?, "node_instance_json")?
  };

  let occupancy = OccupancyChange {
    metrics: OccupancyMetrics {
      connections: decode_u64(field("connections")?, "connections")?,
      publishers: decode_u64(field("publishers")?, "publishers")?,
      subscribers: decode_u64(field("subscribers")?, "subscribers")?,
      presence_connections: decode_u64(field("presence_connections")?, "presence_connections")?,
      presence_subscribers: decode_u64(field("presence_subscribers")?, "presence_subscribers")?,
      presence_members: decode_u64(field("presence_members")?, "presence_members")?,
    },
    changed_categories: decode_categories(&fields, "changed.")?,
    zero_boundary_categories: decode_categories(&fields, "boundary.")?,
  };
  let occupancy = (!occupancy.changed_categories.is_empty()).then_some(occupancy);
  Ok(CommittedPresenceEvent::new(
    event_id,
    PresenceChannelChanged {
      channel: decode_json(field("channel_json")?, "channel_json")?,
      origin,
      presence_revision,
      occupancy_version: decode_u64(field("occupancy_version")?, "occupancy_version")?,
      member_changes,
      occupancy,
      occurred_at,
    },
  ))
}

/// Disconnect либо завершён, либо требует подготовки актуального списка каналов.
pub(super) enum DisconnectReply {
  Complete(Vec<CommittedChannelTransition>),
  Channels(Vec<String>),
}

pub(super) fn decode_disconnect(
  value: &ScriptValue,
) -> Result<DisconnectReply, ChannelStateStoreError> {
  let ScriptValue::Array(values) = value else {
    return Err(invalid("invalid disconnect Lua reply"));
  };
  match values.as_slice() {
    [ScriptValue::Integer(1), ScriptValue::Array(transitions)] => Ok(DisconnectReply::Complete(
      transitions
        .iter()
        .map(decode_transition)
        .collect::<Result<_, _>>()?,
    )),
    [ScriptValue::Bytes(kind), ScriptValue::Array(segments)] if kind == b"channels" => {
      let mut channels = segments
        .iter()
        .map(|segment| {
          let bytes = URL_SAFE_NO_PAD
            .decode(decode_text(segment, "channel segment")?)
            .map_err(|error| invalid(format!("invalid channel segment: {error}")))?;
          String::from_utf8(bytes)
            .map_err(|error| invalid(format!("invalid channel UTF-8: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
      channels.sort_unstable();
      Ok(DisconnectReply::Channels(channels))
    }
    [ScriptValue::Integer(0), code, message] => Err(decode_rejection(
      decode_text(code, "code")?,
      decode_text(message, "message")?.to_owned(),
    )),
    _ => Err(invalid("invalid disconnect Lua reply")),
  }
}

fn invalid(message: impl Into<String>) -> ChannelStateStoreError {
  ChannelStateStoreError::Internal {
    message: message.into(),
  }
}

fn decode_fields(
  value: &ScriptValue,
) -> Result<HashMap<&str, &ScriptValue>, ChannelStateStoreError> {
  let ScriptValue::Array(values) = value else {
    return Err(invalid("expected field/value pairs in Lua reply"));
  };
  let mut fields = HashMap::new();
  for pair in values.chunks(2) {
    let [name, value] = pair else {
      return Err(invalid("incomplete field/value pair in Lua reply"));
    };
    let name = decode_text(name, "field name")?;
    if fields.insert(name, value).is_some() {
      return Err(invalid(format!("duplicate field {name} in Lua reply")));
    }
  }
  Ok(fields)
}

fn required_field<'a>(
  fields: &HashMap<&str, &'a ScriptValue>,
  name: &str,
) -> Result<&'a ScriptValue, ChannelStateStoreError> {
  fields
    .get(name)
    .copied()
    .ok_or_else(|| invalid(format!("missing Lua reply field {name}")))
}

fn decode_categories(
  fields: &HashMap<&str, &ScriptValue>,
  prefix: &str,
) -> Result<BTreeSet<OccupancyCategory>, ChannelStateStoreError> {
  let mut categories = BTreeSet::new();
  for (name, value) in fields {
    if let Some(category) = name.strip_prefix(prefix) {
      if decode_text(value, name)? != "1" {
        return Err(invalid(format!("invalid Occupancy flag {name}")));
      }
      categories.insert(
        OccupancyCategory::from_wire_name(category)
          .ok_or_else(|| invalid(format!("unknown Occupancy category {category}")))?,
      );
    }
  }
  Ok(categories)
}

fn decode_member_data(
  member: &MemberPayload,
) -> Result<Option<serde_json::Value>, ChannelStateStoreError> {
  member
    .data_json
    .as_deref()
    .map(serde_json::from_str)
    .transpose()
    .map_err(|error| invalid(format!("invalid Presence member data: {error}")))
}

/// Разбирает свежий результат и сохранённую HASH-запись при replay.
pub(super) fn decode_presence(
  value: &ScriptValue,
) -> Result<PresenceMutationReceipt, ChannelStateStoreError> {
  let ScriptValue::Array(values) = value else {
    return Err(invalid("invalid Presence Lua reply"));
  };
  let outcome = match values.as_slice() {
    [ScriptValue::Bytes(kind), record] if kind == b"replayed" => {
      let fields = decode_fields(record)?;
      let outcome = match decode_text(required_field(&fields, "result")?, "result")? {
        "committed" => PresenceMutationOutcome::Committed(CommittedChannelTransition::Changed(
          decode_outbox(record)?,
        )),
        "rejected" => PresenceMutationOutcome::Rejected(decode_presence_rejection(
          required_field(&fields, "code")?,
          fields.get("client_id").copied(),
        )?),
        _ => return Err(invalid("invalid saved Presence result")),
      };
      return Ok(PresenceMutationReceipt::replayed(outcome));
    }
    [ScriptValue::Bytes(kind), outbox] if kind == b"committed" => {
      PresenceMutationOutcome::Committed(CommittedChannelTransition::Changed(decode_outbox(
        outbox,
      )?))
    }
    [ScriptValue::Bytes(kind), code, extra @ ..] if kind == b"rejected" && extra.len() <= 1 => {
      PresenceMutationOutcome::Rejected(decode_presence_rejection(code, extra.first())?)
    }
    [ScriptValue::Integer(0), code, message] => {
      return Err(decode_rejection(
        decode_text(code, "code")?,
        decode_text(message, "message")?.to_owned(),
      ));
    }
    _ => return Err(invalid("invalid Presence Lua reply")),
  };
  Ok(PresenceMutationReceipt::fresh(outcome))
}

fn decode_presence_rejection(
  code: &ScriptValue,
  client_id: Option<&ScriptValue>,
) -> Result<PresenceRejection, ChannelStateStoreError> {
  Ok(match decode_text(code, "code")? {
    "notAttached" => PresenceRejection::NotAttached,
    "presenceModeNotEnabled" => PresenceRejection::PresenceModeNotEnabled,
    "unidentifiedConnection" => PresenceRejection::UnidentifiedConnection,
    "clientIdNotAllowed" => PresenceRejection::ClientIdNotAllowed {
      client_id: decode_text(
        client_id.ok_or_else(|| invalid("missing rejected client ID"))?,
        "client_id",
      )?
      .to_owned(),
    },
    "invalidMemberState" => PresenceRejection::InvalidMemberState,
    "conflictingReplay" => PresenceRejection::ConflictingReplay,
    "staleOperation" => PresenceRejection::StaleOperation,
    "connectionClosed" => PresenceRejection::ConnectionClosed,
    code => return Err(invalid(format!("unknown Presence rejection {code}"))),
  })
}

pub(super) fn decode_text<'a>(
  value: &'a ScriptValue,
  field: &str,
) -> Result<&'a str, ChannelStateStoreError> {
  let invalid_text = || ChannelStateStoreError::Internal {
    message: format!("invalid string in Lua reply field {field}"),
  };
  let ScriptValue::Bytes(bytes) = value else {
    return Err(invalid_text());
  };
  std::str::from_utf8(bytes).map_err(|_| invalid_text())
}

fn decode_json<T: DeserializeOwned>(
  value: &ScriptValue,
  field: &str,
) -> Result<T, ChannelStateStoreError> {
  serde_json::from_str(decode_text(value, field)?).map_err(|error| {
    ChannelStateStoreError::Internal {
      message: format!("invalid JSON in Lua reply field {field}: {error}"),
    }
  })
}

/// Проверяет ответ attach и возвращает данные успешной операции.
fn decode_attach_reply(
  value: &ScriptValue,
) -> Result<(&ScriptValue, &ScriptValue), ChannelStateStoreError> {
  let invalid_reply = || ChannelStateStoreError::Internal {
    message: "invalid attach Lua reply".to_owned(),
  };

  let ScriptValue::Array(values) = value else {
    return Err(invalid_reply());
  };

  match values.as_slice() {
    [ScriptValue::Integer(1), snapshot, transition] => Ok((snapshot, transition)),
    [
      ScriptValue::Integer(0),
      ScriptValue::Bytes(code),
      ScriptValue::Bytes(message),
    ] => {
      let code = std::str::from_utf8(code).map_err(|_| invalid_reply())?;
      let message = std::str::from_utf8(message).map_err(|_| invalid_reply())?;

      Err(decode_rejection(code, message.to_owned()))
    }
    _ => Err(invalid_reply()),
  }
}

/// Собирает snapshot и сортирует участников по ID соединения и клиента.
pub(super) fn decode_snapshot(
  value: &ScriptValue,
) -> Result<PresenceSnapshot, ChannelStateStoreError> {
  let invalid_snapshot = || ChannelStateStoreError::Internal {
    message: "invalid snapshot in Lua reply".to_owned(),
  };

  let ScriptValue::Array(values) = value else {
    return Err(invalid_snapshot());
  };
  let [
    ScriptValue::Array(raw_members),
    presence_revision,
    occupancy_version,
    metrics,
  ] = values.as_slice()
  else {
    return Err(invalid_snapshot());
  };

  let mut members = Vec::with_capacity(raw_members.len());
  for value in raw_members {
    let ScriptValue::Array(record) = value else {
      return Err(invalid_snapshot());
    };
    let [payload, revision, timestamp] = record.as_slice() else {
      return Err(invalid_snapshot());
    };
    let payload: MemberPayload = decode_json(payload, "member")?;
    members.push(PresenceMember {
      data: decode_member_data(&payload)?,
      connection_id: payload.connection_id,
      client_id: payload.client_id,
      node_instance: payload.node_instance,
      last_message_id: payload.last_message_id,
      presence_revision: decode_u64(revision, "member revision")?,
      updated_at: Timestamp::from_millis(decode_u64(timestamp, "member timestamp")?),
    });
  }
  members.sort_unstable_by(|left, right| {
    left
      .connection_id
      .as_str()
      .cmp(right.connection_id.as_str())
      .then_with(|| left.client_id.cmp(&right.client_id))
  });

  Ok(PresenceSnapshot {
    members,
    presence_revision: decode_u64(presence_revision, "presence_revision")?,
    occupancy_version: decode_u64(occupancy_version, "occupancy_version")?,
    occupancy: decode_metrics(metrics)?,
  })
}

/// Читает шесть пар field/value в порядке, заданном Lua-контрактом snapshot.
fn decode_metrics(value: &ScriptValue) -> Result<OccupancyMetrics, ChannelStateStoreError> {
  let invalid_metrics = || ChannelStateStoreError::Internal {
    message: "invalid occupancy metrics in Lua reply".to_owned(),
  };
  let ScriptValue::Array(values) = value else {
    return Err(invalid_metrics());
  };
  let fields = [
    "connections",
    "publishers",
    "subscribers",
    "presence_connections",
    "presence_subscribers",
    "presence_members",
  ];
  if values.len() != fields.len() * 2 {
    return Err(invalid_metrics());
  }

  let mut counters = [0; 6];
  for (index, (pair, field)) in values.chunks_exact(2).zip(fields).enumerate() {
    let [ScriptValue::Bytes(name), value] = pair else {
      return Err(invalid_metrics());
    };
    if name.as_slice() != field.as_bytes() {
      return Err(invalid_metrics());
    }
    counters[index] = decode_u64(value, field)?;
  }

  let [
    connections,
    publishers,
    subscribers,
    presence_connections,
    presence_subscribers,
    presence_members,
  ] = counters;
  Ok(OccupancyMetrics {
    connections,
    publishers,
    subscribers,
    presence_connections,
    presence_subscribers,
    presence_members,
  })
}

/// Читает счётчик или версию из десятичной строки ответа Lua.
fn decode_u64(value: &ScriptValue, field: &str) -> Result<u64, ChannelStateStoreError> {
  let invalid_value = || ChannelStateStoreError::Internal {
    message: format!("invalid u64 in Lua reply field {field}"),
  };

  let ScriptValue::Bytes(bytes) = value else {
    return Err(invalid_value());
  };
  let text = std::str::from_utf8(bytes).map_err(|_| invalid_value())?;
  text.parse::<u64>().map_err(|_| invalid_value())
}

/// Преобразует код и сообщение отказа Lua в ошибку хранилища.
pub(super) fn decode_rejection(code: &str, message: String) -> ChannelStateStoreError {
  match code {
    "invalid_request" => ChannelStateStoreError::InvalidRequest { message },
    "lease_lost" | "connection_closed" | "generation_mismatch" | "generation_active" => {
      ChannelStateStoreError::Conflict { message }
    }
    "corrupt_state" | "numeric_overflow" => ChannelStateStoreError::Internal { message },
    _ => ChannelStateStoreError::Internal {
      message: format!("unknown Lua rejection code {code}: {message}"),
    },
  }
}
