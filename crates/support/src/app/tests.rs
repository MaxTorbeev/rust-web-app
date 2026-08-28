use super::read_env::env_value_or;
use super::*;
use std::env::VarError;
use std::error::Error;
use std::ffi::OsString;

#[test]
fn builds_canonical_namespace() {
  let namespace = AppNamespace::try_new("mxt_realtime", "production", "event-bus", 1)
    .expect("namespace must be valid");

  assert_eq!(namespace.as_str(), "mxt_realtime.production.event-bus.v1");
  assert_eq!(namespace.to_string(), namespace.as_str());
  assert_eq!(namespace.as_ref(), namespace.as_str());
}

#[test]
fn accepts_supported_segment_characters() {
  let namespace = AppNamespace::try_new("App_9", "production-eu", "event_bus-2", u64::MAX)
    .expect("supported characters must be accepted");

  assert_eq!(
    namespace.as_str(),
    format!("App_9.production-eu.event_bus-2.v{}", u64::MAX)
  );
}

#[test]
fn rejects_empty_segments() {
  for (app, app_environment, subsystem, expected_field) in [
    ("", "production", "event-bus", "APP"),
    ("mxt_realtime", "", "event-bus", "APP_ENV"),
    ("mxt_realtime", "production", "", "subsystem"),
  ] {
    let error = AppNamespace::try_new(app, app_environment, subsystem, 1)
      .expect_err("empty segment must be rejected");

    assert!(matches!(
      error,
      AppNamespaceError::InvalidNamespaceSegment {
        field,
        reason: "value must not be empty",
        ..
      } if field == expected_field
    ));
  }
}

#[test]
fn rejects_unsupported_segment_characters() {
  for (app, app_environment, subsystem, expected_field) in [
    ("mxt.realtime", "production", "event-bus", "APP"),
    ("mxt_realtime", "production eu", "event-bus", "APP_ENV"),
    ("mxt_realtime", "production", "event.bus", "subsystem"),
    ("mxt_realtime", "production", "event*bus", "subsystem"),
    ("mxt_realtime", "production", "event>bus", "subsystem"),
    ("mxt_realtime", "production", "события", "subsystem"),
  ] {
    let error = AppNamespace::try_new(app, app_environment, subsystem, 1)
      .expect_err("unsupported namespace character must be rejected");

    assert!(matches!(
      error,
      AppNamespaceError::InvalidNamespaceSegment { field, .. }
        if field == expected_field
    ));
  }
}

#[test]
fn rejects_zero_version() {
  let error = AppNamespace::try_new("mxt_realtime", "production", "event-bus", 0)
    .expect_err("zero namespace version must be rejected");

  assert_eq!(error, AppNamespaceError::InvalidVersion { version: 0 });
}

#[test]
fn read_env_error_retains_variable_and_source() {
  let error = ReadEnvError::new("APP", VarError::NotPresent);

  assert_eq!(error.variable(), "APP");
  assert_eq!(error.var_error(), &VarError::NotPresent);
  assert_eq!(
    error
      .source()
      .and_then(|source| source.downcast_ref::<VarError>()),
    Some(&VarError::NotPresent)
  );
}

#[test]
fn app_namespace_error_wraps_read_env_error_as_its_source() {
  let read_error = ReadEnvError::new("APP", VarError::NotPresent);
  let error = AppNamespaceError::from(read_error.clone());

  assert_eq!(
    error
      .source()
      .and_then(|source| source.downcast_ref::<ReadEnvError>()),
    Some(&read_error)
  );
}

#[test]
fn read_env_or_uses_default_when_variable_is_not_present() {
  let value = env_value_or(
    Err(ReadEnvError::new("APP_ENV", VarError::NotPresent)),
    "development",
  )
  .expect("missing variable must use its default");

  assert_eq!(value, "development");
}

#[test]
fn read_env_or_preserves_empty_value() {
  let value = env_value_or(Ok(String::new()), "development")
    .expect("present empty variable must remain a valid value");

  assert!(value.is_empty());
}

#[test]
fn read_env_or_returns_not_unicode_error() {
  let source = VarError::NotUnicode(OsString::from("invalid value"));
  let expected = ReadEnvError::new("APP_ENV", source);

  let error = env_value_or(Err(expected.clone()), "development")
    .expect_err("non-Unicode variable must remain an error");

  assert_eq!(error, expected);
}
