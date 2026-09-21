//! Build the workspace binary first. Requires disposable Redis and JetStream.
//! REALTIME_REDIS_TEST_PORT=16389 REALTIME_NATS_TEST_URL=nats://127.0.0.1:14222 \
//!   cargo test -p realtime --test redis_cluster -- --ignored --nocapture
use auth::TokenAccessIssuer;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{
  path::PathBuf,
  process::{Child, Command, Stdio},
  time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
struct Node {
  child: Child,
  port: u16,
}
impl Drop for Node {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}
impl Node {
  fn start(
    root: &std::path::Path,
    directory: &std::path::Path,
    name: &str,
    namespace: &str,
  ) -> Self {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
      .unwrap()
      .local_addr()
      .unwrap()
      .port();
    let log = std::fs::File::create(directory.join(format!("{name}-{port}.log"))).unwrap();
    let child = Command::new(root.join("target/debug/mxt-realtime"))
      .current_dir(directory)
      .env_clear()
      .env("PATH", std::env::var("PATH").unwrap_or_default())
      .env("APP", "presence_cluster_test")
      .env("APP_ENV", namespace)
      .env("APP_NODE_ID", name)
      .env("APP_URL", format!("127.0.0.1:{port}"))
      .env("APP_REALTIME_API_KEY", "test.key:test-secret")
      .env("APP_AUTH_LOGIN", "unused")
      .env("APP_AUTH_PASSWORD_HASH", "unused")
      .env("REDIS_HOST", "127.0.0.1")
      .env(
        "REDIS_PORT",
        std::env::var("REALTIME_REDIS_TEST_PORT").expect("Redis port"),
      )
      .env("PRESENCE_STORE_DRIVER", "redis")
      .env("EVENT_BUS_DRIVER", "nats")
      .env(
        "NATS_SERVERS",
        std::env::var("REALTIME_NATS_TEST_URL").expect("NATS URL"),
      )
      .env("NATS_STREAM_NAME", format!("PRESENCE_TEST_{namespace}"))
      .env("RUST_LOG", "info")
      .stdout(Stdio::from(log.try_clone().unwrap()))
      .stderr(Stdio::from(log))
      .spawn()
      .unwrap();
    Self { child, port }
  }
  async fn connect(&mut self, client: &str) -> Socket {
    let issuer = TokenAccessIssuer::new("test.key", b"test-secret");
    let capability = r#"{"*":["subscribe","presence","publish"]}"#.parse().unwrap();
    let token = issuer
      .issue(Some(client.to_owned()), &capability, 120)
      .unwrap();
    for _ in 0..100 {
      assert!(
        self.child.try_wait().unwrap().is_none(),
        "node exited; inspect temporary logs"
      );
      if let Ok((mut socket, _)) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{}/?access_token={token}&format=json&heartbeats=false",
        self.port
      ))
      .await
      {
        receive(&mut socket, 4, Duration::from_secs(3)).await;
        return socket;
      }
      tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("node failed to start")
  }
}
async fn send(socket: &mut Socket, value: Value) {
  socket
    .send(Message::Text(value.to_string().into()))
    .await
    .unwrap();
}
async fn receive(socket: &mut Socket, action: u64, timeout: Duration) -> Value {
  tokio::time::timeout(timeout, async {
    loop {
      let message = socket.next().await.expect("socket ended").unwrap();
      if let Message::Text(text) = message {
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["action"] == action {
          return value;
        }
        assert!(value["action"] != 2, "unexpected NACK: {value}");
      }
    }
  })
  .await
  .expect("protocol response timed out")
}
async fn attach(socket: &mut Socket) -> Value {
  send(socket, json!({"action":10,"channel":"room"})).await;
  receive(socket, 11, Duration::from_secs(3)).await;
  receive(socket, 16, Duration::from_secs(3)).await
}

#[tokio::test]
#[ignore = "requires built application, disposable Redis and JetStream"]
async fn two_nodes_deliver_retry_reap_restart_and_stop_on_redis_outage() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .canonicalize()
    .unwrap();
  let namespace = uuid::Uuid::new_v4().simple().to_string();
  let directory = std::env::temp_dir().join(format!("presence-cluster-{namespace}"));
  std::fs::create_dir_all(directory.join("config")).unwrap();
  std::fs::copy(
    root.join("config/event_bus.toml"),
    directory.join("config/event_bus.toml"),
  )
  .unwrap();
  eprintln!("node logs: {}", directory.display());
  let mut a = Node::start(&root, &directory, "node-a", &namespace);
  let mut alice = a.connect("alice").await;
  let mut b = Node::start(&root, &directory, "node-b", &namespace);
  let mut bob = b.connect("bob").await;
  assert!(
    attach(&mut alice).await["presence"]
      .as_array()
      .unwrap()
      .is_empty()
  );
  attach(&mut bob).await;
  let enter = json!({"action":14,"channel":"room","msgSerial":0,"presence":[{"action":2,"clientId":"alice","data":{"large":18446744073709551615u64}}]});
  send(&mut alice, enter.clone()).await;
  let remote = receive(&mut bob, 14, Duration::from_secs(5)).await;
  assert_eq!(remote["presence"][0]["clientId"], "alice");
  assert_eq!(remote["presence"][0]["action"], 2);
  assert_eq!(
    remote["presence"][0]["data"]["large"],
    json!(18446744073709551615u64)
  );
  // Both ACK and self-delivery may arrive in either order.
  tokio::time::timeout(Duration::from_secs(5), async {
    let mut ack = false;
    let mut presence = false;
    while !ack || !presence {
      let Message::Text(text) = alice.next().await.unwrap().unwrap() else {
        continue;
      };
      let value: Value = serde_json::from_str(&text).unwrap();
      ack |= value["action"] == 1;
      presence |= value["action"] == 14;
      assert!(value["action"] != 2, "{value}");
    }
  })
  .await
  .unwrap();
  send(&mut alice, enter).await;
  receive(&mut alice, 1, Duration::from_secs(3)).await;
  assert!(
    tokio::time::timeout(Duration::from_millis(700), bob.next())
      .await
      .is_err(),
    "retry emitted a second event"
  );
  let mut observer = b.connect("observer").await;
  assert_eq!(
    attach(&mut observer).await["presence"][0]["clientId"],
    "alice"
  );
  a.child.kill().unwrap();
  a.child.wait().unwrap();
  let leave = receive(&mut bob, 14, Duration::from_secs(22)).await;
  assert_eq!(leave["presence"][0]["action"], 3);
  assert!(
    leave["presence"][0]["id"]
      .as_str()
      .unwrap()
      .starts_with("server:")
  );
  let mut restarted = Node::start(&root, &directory, "node-a", &namespace);
  let mut alice = restarted.connect("alice").await;
  assert!(
    attach(&mut alice).await["presence"]
      .as_array()
      .unwrap()
      .is_empty()
  );
  send(&mut alice, json!({"action":14,"channel":"room","msgSerial":0,"presence":[{"action":2,"clientId":"alice"}]})).await;
  assert_eq!(
    receive(&mut bob, 14, Duration::from_secs(5)).await["presence"][0]["action"],
    2
  );
  send(&mut alice, json!({"action":12,"channel":"room"})).await;
  receive(&mut alice, 13, Duration::from_secs(3)).await;
  assert_eq!(
    receive(&mut bob, 14, Duration::from_secs(5)).await["presence"][0]["action"],
    3
  );
  attach(&mut alice).await;
  send(&mut alice, json!({"action":14,"channel":"room","msgSerial":1,"presence":[{"action":2,"clientId":"alice"}]})).await;
  assert_eq!(
    receive(&mut bob, 14, Duration::from_secs(5)).await["presence"][0]["action"],
    2
  );
  send(&mut alice, json!({"action":5})).await;
  assert_eq!(
    receive(&mut bob, 14, Duration::from_secs(5)).await["presence"][0]["action"],
    3
  );

  // Only run against a disposable Redis: delay all commands beyond response timeout.
  let pause = Command::new("redis-cli")
    .args([
      "-p",
      &std::env::var("REALTIME_REDIS_TEST_PORT").unwrap(),
      "CLIENT",
      "PAUSE",
      "10000",
      "ALL",
    ])
    .output()
    .unwrap();
  assert!(pause.status.success());
  assert_eq!(String::from_utf8_lossy(&pause.stdout).trim(), "OK");
  send(
    &mut bob,
    json!({"action":14,"channel":"room","msgSerial":0,"presence":[{"action":2,"clientId":"bob"}]}),
  )
  .await;
  tokio::time::timeout(Duration::from_secs(10), async {
    while let Some(Ok(message)) = bob.next().await {
      if let Message::Text(text) = message {
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_ne!(value["action"], 1, "Redis outage produced a successful ACK");
      }
    }
    loop {
      if b.child.try_wait().unwrap().is_some() && restarted.child.try_wait().unwrap().is_some() {
        break;
      }
      tokio::time::sleep(Duration::from_millis(100)).await;
    }
  })
  .await
  .expect("nodes must stop after Redis failure");
}
