//! Интеграционные тесты сессии запускают настоящий WebSocket через локальный Axum-сервер.
//! Новые сценарии можно собирать из `send`, `expect_action` и проверок состояния.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use auth::VerifiedToken;
use axum::extract::ws::WebSocketUpgrade;
use axum::routing::any;
use axum::Router;
use event_bus::{Event, EventBus};
use futures_util::{SinkExt, StreamExt};
use realtime::{
  register_event_handlers,
  ApplicationId,
  Connection,
  ConnectionId,
  PresenceAction,
  ProtocolAction,
  ProtocolMessage,
  Realtime,
  RealtimeApplication,
  RealtimeConfig,
  WebsocketConnected,
  WebsocketDisconnected,
};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const TEST_TIMEOUT: Duration = Duration::from_secs(1);
// EventBus локальный: короткого окна достаточно, чтобы поймать повторную отправку.
const NO_EVENT_TIMEOUT: Duration = Duration::from_millis(50);
const CHANNEL: &str = "session-cleanup";

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "current_thread")]
async fn disconnect_cleans_session_state_and_emits_lifecycle_once() {
  let mut session = TestSession::connect().await;
  let connection_id = session.connection_id.clone();

  let connected = session
    .expect_action(ProtocolAction::Connected)
    .await;
  assert_eq!(
    connected.connection_id.as_deref(),
    Some(connection_id.as_str()),
  );
  session.events.expect_connected(&connection_id).await;

  // Создаём состояние channel и presence, которое session должна очистить.
  session.send(json!({
    "action": ProtocolAction::Attach,
    "channel": CHANNEL,
  })).await;
  session.expect_action(ProtocolAction::Attached).await;
  session.expect_action(ProtocolAction::Sync).await;

  session.send(json!({
    "action": ProtocolAction::Presence,
    "channel": CHANNEL,
    "msgSerial": 1,
    "presence": [{ "action": PresenceAction::Enter }],
  })).await;
  session.expect_action(ProtocolAction::Presence).await;
  session.expect_action(ProtocolAction::Ack).await;

  // Без этих предусловий последующая проверка cleanup ничего не доказывает.
  assert!(session.is_attached(CHANNEL).await);
  assert_eq!(session.presence_count(CHANNEL).await, 1);

  session.send(json!({
    "action": ProtocolAction::Disconnect,
  })).await;
  session.expect_action(ProtocolAction::Disconnected).await;
  session.events.expect_disconnected(&connection_id).await;

  // После finish ни channel, ни presence не должны хранить connection.
  assert!(!session.is_attached(CHANNEL).await);
  assert_eq!(session.presence_count(CHANNEL).await, 0);
  session.events.expect_no_more_events().await;
}

#[tokio::test(flavor = "current_thread")]
async fn channel_message_is_broadcast_and_acknowledged() {
  let mut session = TestSession::connect().await;
  let connection_id = session.connection_id.clone();

  session.expect_action(ProtocolAction::Connected).await;
  session.events.expect_connected(&connection_id).await;

  session.send(json!({
    "action": ProtocolAction::Attach,
    "channel": CHANNEL,
  })).await;
  session.expect_action(ProtocolAction::Attached).await;
  session.expect_action(ProtocolAction::Sync).await;

  session.send(json!({
    "action": ProtocolAction::Message,
    "channel": CHANNEL,
    "msgSerial": 7,
    "messages": [{
      "name": "chat.message",
      "data": { "text": "hello" },
    }],
  })).await;

  let published = session
    .expect_action(ProtocolAction::Message)
    .await;
  assert_eq!(published.channel.as_deref(), Some(CHANNEL));
  assert_eq!(published.msg_serial, None);

  let messages = published
    .messages
    .expect("broadcast must preserve published messages");
  assert_eq!(messages.len(), 1);
  assert_eq!(messages[0].name.as_deref(), Some("chat.message"));
  assert_eq!(messages[0].data, json!({ "text": "hello" }));

  let ack = session.expect_action(ProtocolAction::Ack).await;
  assert_eq!(ack.msg_serial, Some(7));
  assert_eq!(ack.count, Some(1));
}

/// Собирает lifecycle events отдельно от сообщений WebSocket-протокола.
struct LifecycleEvents {
  connected: UnboundedReceiver<WebsocketConnected>,
  disconnected: UnboundedReceiver<WebsocketDisconnected>,
}

impl LifecycleEvents {
  fn register(event_bus: &mut EventBus) -> Self {
    let connected = register_handler::<WebsocketConnected>(event_bus);
    let disconnected = register_handler::<WebsocketDisconnected>(event_bus);

    Self {
      connected,
      disconnected,
    }
  }

  async fn expect_connected(&mut self, connection_id: &ConnectionId) {
    let event = receive_event(
      &mut self.connected,
      "WebsocketConnected",
    ).await;

    assert_eq!(event.connection_id, connection_id.as_str());
  }

  async fn expect_disconnected(&mut self, connection_id: &ConnectionId) {
    let event = receive_event(
      &mut self.disconnected,
      "WebsocketDisconnected",
    ).await;

    assert_eq!(event.connection_id, connection_id.as_str());
  }

  async fn expect_no_more_events(&mut self) {
    tokio::select! {
      Some(event) = self.connected.recv() => {
        panic!("unexpected additional WebsocketConnected event: {event:?}");
      }

      Some(event) = self.disconnected.recv() => {
        panic!("unexpected additional WebsocketDisconnected event: {event:?}");
      }

      _ = sleep(NO_EVENT_TIMEOUT) => {}
    }
  }
}

/// Тестовое окружение для одного реального WebSocket connection.
/// Сервер автоматически останавливается при завершении теста.
struct TestSession {
  socket: ClientSocket,
  application: Arc<RealtimeApplication>,
  connection_id: ConnectionId,
  events: LifecycleEvents,
  server: JoinHandle<()>,
}

impl TestSession {
  async fn connect() -> Self {
    let mut event_bus = EventBus::new();
    let events = LifecycleEvents::register(&mut event_bus);
    let realtime = test_realtime();
    let application = realtime
      .application(&test_application_id())
      .expect("test realtime application must exist");

    register_event_handlers(&mut event_bus, realtime)
      .expect("realtime event handlers must register");

    let event_bus = Arc::new(event_bus);
    let connection = application.create_connection(test_authorization());
    let connection_id = connection.id.clone();
    let router = single_connection_router(
      connection,
      application.clone(),
      event_bus,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("test server must bind");
    let address = listener
      .local_addr()
      .expect("test server must have an address");
    let server = tokio::spawn(async move {
      axum::serve(listener, router)
        .await
        .expect("test server must run");
    });

    let (socket, _) =
      tokio_tungstenite::connect_async(format!("ws://{address}/"))
        .await
        .expect("test client must connect");

    Self {
      socket,
      application,
      connection_id,
      events,
      server,
    }
  }

  async fn send(&mut self, message: Value) {
    self
      .socket
      .send(Message::Text(message.to_string().into()))
      .await
      .expect("protocol message must be sent");
  }

  async fn expect_action(
    &mut self,
    expected_action: ProtocolAction,
  ) -> ProtocolMessage {
    let message = self.receive().await;
    let actual_code = message.action.clone() as u8;
    let expected_code = expected_action as u8;

    assert_eq!(
      actual_code,
      expected_code,
      "unexpected protocol action",
    );

    message
  }

  async fn is_attached(&self, channel: &str) -> bool {
    self
      .application
      .channel_hub
      .is_attached(channel, &self.connection_id)
      .await
  }

  async fn presence_count(&self, channel: &str) -> usize {
    self
      .application
      .presence_hub
      .snapshot(channel)
      .await
      .len()
  }

  async fn receive(&mut self) -> ProtocolMessage {
    let frame = timeout(TEST_TIMEOUT, self.socket.next())
      .await
      .expect("the session must respond")
      .expect("the WebSocket must remain open")
      .expect("the WebSocket frame must be readable");

    let Message::Text(text) = frame else {
      panic!("expected a text protocol frame");
    };

    serde_json::from_str(text.as_str())
      .expect("the frame must contain a protocol message")
  }
}

impl Drop for TestSession {
  fn drop(&mut self) {
    self.server.abort();
  }
}

fn register_handler<E: Event>(
  event_bus: &mut EventBus,
) -> UnboundedReceiver<E> {
  let (sender, receiver) = mpsc::unbounded_channel();
  event_bus
    .register(move |event: E| {
      let sender = sender.clone();

      async move {
        let _ = sender.send(event);

        Ok(())
      }
    })
    .expect("lifecycle handler must register");

  receiver
}

async fn receive_event<E>(
  receiver: &mut UnboundedReceiver<E>,
  event_name: &str,
) -> E {
  timeout(TEST_TIMEOUT, receiver.recv())
    .await
    .unwrap_or_else(|_| panic!("{event_name} event must be emitted"))
    .unwrap_or_else(|| panic!("{event_name} event channel must stay open"))
}

fn single_connection_router(
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) -> Router {
  let pending_connection = Arc::new(Mutex::new(Some(connection)));

  Router::new().route(
    "/",
    any(move |ws: WebSocketUpgrade| {
      let application = application.clone();
      let event_bus = event_bus.clone();
      let connection = pending_connection
        .lock()
        .expect("test connection lock must not be poisoned")
        .take()
        .expect("the test accepts exactly one WebSocket");

      async move {
        ws.on_upgrade(move |socket| {
          realtime::websocket::handle_socket(
            socket,
            connection,
            application,
            event_bus,
          )
        })
      }
    }),
  )
}

fn test_application_id() -> ApplicationId {
  ApplicationId::new("application-1")
}

fn test_realtime() -> Arc<Realtime> {
  Arc::new(Realtime::from_config(RealtimeConfig {
    application_id: test_application_id(),
    key_name: "application-1.test-key".to_owned(),
    key_secret: "test-secret".to_owned(),
  }))
}

fn test_authorization() -> VerifiedToken {
  VerifiedToken {
    client_id: Some("client-123".to_owned()),
    issued_at: 1,
    expires_at: 2,
    capability: r#"{"*": ["subscribe", "presence"]}"#
      .parse()
      .expect("test capability must be valid"),
  }
}
