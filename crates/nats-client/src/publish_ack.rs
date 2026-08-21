use async_nats::jetstream::publish::PublishAck as DriverPublishAck;

/// JetStream confirmation that a message was accepted by a stream.
///
/// It does not confirm consumer processing or delivery to an application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishAck {
    pub stream: String,
    pub sequence: u64,
    pub domain: String,
    pub duplicate: bool,
    pub value: Option<String>,
}

impl PublishAck {
    pub(crate) fn from_driver(ack: DriverPublishAck) -> Self {
        Self {
            stream: ack.stream,
            sequence: ack.sequence,
            domain: ack.domain,
            duplicate: ack.duplicate,
            value: ack.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_driver_ack_without_exposing_it() {
        let ack = PublishAck::from_driver(DriverPublishAck {
            stream: "EVENTS".to_owned(),
            sequence: 42,
            domain: "production".to_owned(),
            duplicate: true,
            value: Some("7".to_owned()),
        });

        assert_eq!(
            ack,
            PublishAck {
                stream: "EVENTS".to_owned(),
                sequence: 42,
                domain: "production".to_owned(),
                duplicate: true,
                value: Some("7".to_owned()),
            },
        );
    }
}
