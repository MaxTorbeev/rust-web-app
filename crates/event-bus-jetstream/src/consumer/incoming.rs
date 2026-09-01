use std::future::poll_fn;
use std::sync::Arc;

use event_bus::{EventMessage, IncomingEventOutcome, IncomingEventProcessor, ProcessingErrorClass};
use nats_client::{NatsMessage, NatsSubscription};

use crate::consumer::JetStreamIncomingConsumerConfig;
use crate::health::{HealthCheck, HealthLifecycle};

use super::error::JetStreamConsumerError;
use super::settlement::SettlementAction;

/// Processes durable JetStream deliveries and applies their settlement action.
///
/// The consumer decodes each delivery into an event envelope, delegates domain
/// processing to [`IncomingEventProcessor`], and then sends `ACK`, `NAK`, or
/// `TERM` to JetStream. It does not spawn or supervise its own task; the
/// application must run and supervise [`Self::run`].
///
/// Lifecycle health starts as [`crate::health::HealthState::Starting`] and
/// becomes `Running` only after the underlying delivery stream is polled.
pub struct JetStreamIncomingConsumer {
  processor: Arc<IncomingEventProcessor>,
  subscription: NatsSubscription,
  config: JetStreamIncomingConsumerConfig,
  lifecycle: HealthLifecycle,
}

impl JetStreamIncomingConsumer {
  /// Creates an incoming consumer without starting its receive loop.
  ///
  /// The returned consumer remains in `Starting` health until [`Self::run`] is
  /// polled by its supervisor.
  pub fn new(
    processor: Arc<IncomingEventProcessor>,
    subscription: NatsSubscription,
    config: JetStreamIncomingConsumerConfig,
  ) -> Self {
    Self {
      processor,
      subscription,
      config,
      lifecycle: HealthLifecycle::new(),
    }
  }

  /// Returns a cloneable read handle for the consumer lifecycle state.
  ///
  /// Obtain this handle before moving the consumer into [`Self::run`].
  pub fn health_check(&self) -> HealthCheck {
    self.lifecycle.health_check()
  }

  /// Runs the receive, processing, and settlement loop.
  ///
  /// A terminal receive or settlement failure changes lifecycle health to
  /// `Failed` before the error is returned. Cancelling or dropping this future
  /// changes it to `Stopped`.
  ///
  /// # Errors
  ///
  /// Returns [`JetStreamConsumerError`] when the subscription closes, receiving
  /// a delivery fails, or JetStream rejects an `ACK`, `NAK`, or `TERM` action.
  pub async fn run(mut self) -> Result<(), JetStreamConsumerError> {
    let result = self.run_loop().await;

    if result.is_err() {
      self.lifecycle.fail();
    }

    result
  }

  async fn run_loop(&mut self) -> Result<(), JetStreamConsumerError> {
    while let Some(delivery) = self.next_delivery().await {
      let delivery = delivery.map_err(JetStreamConsumerError::Receive)?;

      self.handle(delivery).await?;
    }

    Err(JetStreamConsumerError::SubscriptionClosed)
  }

  async fn next_delivery(&mut self) -> Option<Result<NatsMessage, nats_client::ReceiveError>> {
    let subscription = &mut self.subscription;
    let lifecycle = &self.lifecycle;

    poll_fn(|context| {
      let poll = subscription.poll_next(context);
      lifecycle.observe_first_poll(&poll);
      poll
    })
    .await
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
            delay: self.config.retry_delay(),
          },
          ProcessingErrorClass::Permanent => SettlementAction::Terminate,
        }
      }
    }
  }
}
