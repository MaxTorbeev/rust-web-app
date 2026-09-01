mod live_health_response;
mod ready_health_response;
mod release_response;

pub(crate) use live_health_response::LiveHealthResponse;
pub(crate) use ready_health_response::ReadyHealthResponse;
pub(crate) use release_response::ReleaseResponse;

const HEALTH_SCHEMA_VERSION: u8 = 1;
