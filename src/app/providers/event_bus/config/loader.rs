use crate::app::providers::EventBusConfigSource;
use confique::Config;

pub(in crate::app::providers::event_bus) fn load() -> Result<EventBusConfigSource, confique::Error>
{
  EventBusConfigSource::builder()
    .env()
    .file("config/event_bus.toml")
    .load()
}
