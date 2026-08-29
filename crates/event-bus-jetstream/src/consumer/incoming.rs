use std::sync::Arc;
use std::time::Duration;

use event_bus::{EventMessage, IncomingEventOutcome, IncomingEventProcessor, ProcessingErrorClass};
use nats_client::{NatsMessage, NatsSubscription};

use super::error::JetStreamConsumerError;
use super::settlement::SettlementAction;

pub struct JetStreamIncomingConsumer {
  processor: Arc<IncomingEventProcessor>,
  retry_delay: Duration,
}

impl JetStreamIncomingConsumer {
  pub fn new(processor: Arc<IncomingEventProcessor>, retry_delay: Duration) -> Self {
    Self {
      processor,
      retry_delay,
    }
  }

  pub async fn run(&self, mut subscription: NatsSubscription) -> Result<(), JetStreamConsumerError> {
    while let Some(delivery) = subscription.next().await {
      let delivery = delivery.map_err(JetStreamConsumerError::Receive)?;

      self.handle(delivery).await?;
    }

    Err(JetStreamConsumerError::SubscriptionClosed)
  }

  async fn handle(&self, delivery: NatsMessage) -> Result<(), JetStreamConsumerError> {
    let settlement = self.decide(delivery.payload()).await;

    settlement.apply(&delivery).await
  }

  async fn decide(&self, payload: &[u8]) -> SettlementAction {
    let message = match EventMessage::from_bytes(payload) {
      Ok(message) => message,
      Err(error) => {
        tracing::error!(
          error = %error,
          "invalid incoming event envelope"
        );

        return SettlementAction::Terminate;
      }
    };

    match self.processor.process(&message).await {
      Ok(IncomingEventOutcome::Applied) => SettlementAction::Ack,
      Ok(IncomingEventOutcome::Duplicate) => SettlementAction::Ack,
      Ok(IncomingEventOutcome::InProgress { retry_after }) => {
        SettlementAction::Nak { delay: retry_after }
      }

      Err(error) => {
        tracing::warn!(
          event_id = %message.event_id(),
          error = %error,
          "incoming event processing failed"
        );

        match error.class() {
          ProcessingErrorClass::Retryable => SettlementAction::Nak {
            delay: self.retry_delay,
          },
          ProcessingErrorClass::Permanent => SettlementAction::Terminate,
        }
      }
    }
  }
}
