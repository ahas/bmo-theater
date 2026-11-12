//! What the tests share: a database of each test's own on the test server, the service started over it in this
//! process with a file root of its own, calls of its routes, rows seeded straight into its tables, and videos
//! made with ffmpeg where it is installed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bmo_theater::settings::Settings;
use reqwest::Method;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{Connection, Executor, PgPool};
use tokio::sync::oneshot;

/// The service token.
pub const TOKEN: &str = "service-token";
/// The console's credential.
pub const CONSOLE: &str = "console-token";
/// Two accounts, and the administrator's.
pub const MEMBER_A: &str = "2";
pub const MEMBER_B: &str = "3";
pub const ADMIN: &str = "1";

static COUNT: AtomicU64 = AtomicU64::new(0);

/// The test server, as the suite names it, and the machine's own socket where nothing does.
pub fn test_address() -> String {
  std::env::var("BMO_TEST_POSTGRES")
    .ok()
    .filter(|named| !named.is_empty())
    .unwrap_or_else(|| "postgres:///bmo_test?host=/var/run/postgresql".into())
}

/// The test server's address with another database in place of its own.
pub fn address_of(database: &str) -> String {
  let address = test_address();
  let (head, query) = match address.split_once('?') {
    Some((head, query)) => (head.to_owned(), format!("?{query}")),
    None => (address.clone(), String::new()),
  };
  let base = match head.strip_prefix("postgres://").or_else(|| head.strip_prefix("postgresql://")) {
    Some(rest) => match rest.split_once('/') {
      Some((authority, _)) => format!("postgres://{authority}"),
      None => format!("postgres://{rest}"),
    },
    None => head,
  };
  format!("{base}/{database}{query}")
}

async fn admin() -> PgConnection {
  let options = PgConnectOptions::from_str(&address_of("postgres")).expect("the server's address reads");
  PgConnection::connect_with(&options).await.expect("the test server answers")
}

/// A fresh folder of the test's own.
pub fn scratch() -> PathBuf {
  let folder = std::env::temp_dir().join(format!(
    "bmo-theater-test-{}-{}",
    std::process::id(),
    COUNT.fetch_add(1, Ordering::Relaxed)
  ));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).unwrap();
  folder
}

/// Whether ffmpeg and ffprobe run here.
pub fn ffmpeg_here() -> bool {
  ["ffmpeg", "ffprobe"].iter().all(|command| {
    Command::new(command)
      .arg("-version")
      .output()
      .is_ok_and(|output| output.status.success())
  })
}

/// A one-second test video with sound, written by ffmpeg.
pub fn make_video(path: &Path) {
  if let Some(folder) = path.parent() {
    std::fs::create_dir_all(folder).unwrap();
  }
  let output = Command::new("ffmpeg")
    .args([
      "-nostdin",
      "-v",
      "error",
      "-f",
      "lavfi",
      "-i",
      "testsrc=duration=1:size=160x120:rate=10",
    ])
    .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
    .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", "-y"])
    .arg(path)
    .output()
    .expect("ffmpeg runs");
  assert!(
    output.status.success(),
    "ffmpeg made no video: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

/// The service running over a database and a file root of its own.
pub struct Service {
  pub address: String,
  pub folder: PathBuf,
  pub pool: PgPool,
  database: String,
  stop: Option<oneshot::Sender<()>>,
  ended: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl Service {
  /// The service started, once it answers ready; `bundle` writes a console bundle into its folder first.
  pub async fn start_with(bundle: Option<Value>) -> Self {
    Self::start_tuned(bundle, |_| ()).await
  }

  /// The service started as `start_with` starts it, its settings changed by `tune` before it starts.
  pub async fn start_tuned(bundle: Option<Value>, tune: impl FnOnce(&mut Settings)) -> Self {
    let folder = scratch();
    let database = format!("bmo_rs_copy_{}_{}", std::process::id(), COUNT.fetch_add(1, Ordering::Relaxed));
    let mut admin = admin().await;
    admin
      .execute(format!("DROP DATABASE IF EXISTS \"{database}\" WITH (FORCE)").as_str())
      .await
      .unwrap();
    admin
      .execute(format!("CREATE DATABASE \"{database}\" TEMPLATE template0 ENCODING 'UTF8'").as_str())
      .await
      .unwrap();
    admin.close().await.unwrap();
    if let Some(words) = bundle {
      let dist = folder.join("bundle");
      std::fs::create_dir_all(&dist).unwrap();
      std::fs::write(dist.join("bundle.js"), "export default [];").unwrap();
      std::fs::write(dist.join("bundle.css"), ".theater {}").unwrap();
      std::fs::write(dist.join("words.json"), words.to_string()).unwrap();
    }
    let mut settings = Settings::read(&folder, Some(Default::default())).unwrap();
    settings.server.token = TOKEN.into();
    settings.console.token = CONSOLE.into();
    settings.console.folder = folder.join("bundle");
    settings.database.url = address_of(&database);
    settings.files.root = folder.join("root");
    settings.share.address = "https://console.test/theater/share".into();
    tune(&mut settings);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = oneshot::channel();
    // The service runs on a runtime of its own, as its binary does, so its future need not be sent between threads.
    let ended = std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
      runtime.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        bmo_theater::serve_on(settings, listener, async move {
          let _ = stopped.await;
        })
        .await
      })
    });
    let pool = PgPool::connect(&address_of(&database)).await.unwrap();
    let service = Self {
      address,
      folder,
      pool,
      database,
      stop: Some(stop),
      ended: Some(ended),
    };
    for _ in 0..500 {
      if service.ended.as_ref().is_some_and(|ended| ended.is_finished()) {
        panic!("the service ended as it started");
      }
      if let Ok(response) = reqwest::Client::new()
        .get(format!("{}/health", service.address))
        .bearer_auth(TOKEN)
        .send()
        .await
        && response.json::<Value>().await.is_ok_and(|health| health["ready"] == true)
      {
        return service;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the service never answered ready");
  }

  pub async fn start() -> Self {
    Self::start_with(None).await
  }

  /// Stops the service, and drops its database.
  pub async fn stop(mut self) {
    if let Some(stop) = self.stop.take() {
      let _ = stop.send(());
    }
    if let Some(ended) = self.ended.take() {
      tokio::task::spawn_blocking(move || ended.join()).await.unwrap().unwrap().unwrap();
    }
    self.pool.close().await;
    let mut admin = admin().await;
    let _ = admin
      .execute(format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.database).as_str())
      .await;
    let _ = std::fs::remove_dir_all(&self.folder);
  }

  /// One call with the given bearer and headers: its status, its headers and its body as JSON or as text.
  pub async fn raw(
    &self,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
    body: Option<Value>,
  ) -> reqwest::Response {
    let mut request = reqwest::Client::new().request(method, format!("{}{path}", self.address));
    if let Some(bearer) = bearer {
      request = request.bearer_auth(bearer);
    }
    for (name, value) in headers {
      request = request.header(*name, *value);
    }
    if let Some(body) = body {
      request = request.json(&body);
    }
    request.send().await.unwrap()
  }

  pub async fn ask(
    &self,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
    body: Option<Value>,
  ) -> (u16, Value) {
    let response = self.raw(method, path, bearer, headers, body).await;
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
  }

  /// A call with the service token, acting for no account.
  pub async fn service(&self, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self.ask(method, path, Some(TOKEN), &[], body).await
  }

  /// A call through the console for a member.
  pub async fn member(&self, key: &str, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self
      .ask(method, path, Some(CONSOLE), &[("Owner-Key", key), ("Owner-Role", "member")], body)
      .await
  }

  /// A call through the console for the administrator.
  pub async fn admin(&self, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self
      .ask(
        method,
        path,
        Some(CONSOLE),
        &[("Owner-Key", ADMIN), ("Owner-Role", "administrator")],
        body,
      )
      .await
  }

  /// A share read: the console's credential with no owner key and no role.
  pub async fn shared(&self, path: &str, headers: &[(&str, &str)]) -> reqwest::Response {
    self.raw(Method::GET, path, Some(CONSOLE), headers, None).await
  }

  /// Waits until a condition on the service's answers holds.
  pub async fn until(&self, mut holds: impl AsyncFnMut(&Self) -> bool) {
    for _ in 0..3000 {
      if holds(self).await {
        return;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the condition never held");
  }

  /// A title seeded straight into the tables.
  pub async fn seed_title(&self, realm: &str, owner: &str, name: &str) -> i64 {
    sqlx::query_scalar(
      "INSERT INTO titles (realm, owner, creator, name, kind, created_at, changed_at) \
       VALUES ($1, $2, $2, $3, 'show', 1, 1) RETURNING id",
    )
    .bind(realm)
    .bind(owner)
    .bind(name)
    .fetch_one(&self.pool)
    .await
    .unwrap()
  }

  /// An episode seeded straight into the tables, its file a run of `bytes` bytes in the test's folder.
  pub async fn seed_episode(&self, title: i64, number: i32, duration_ms: i64, bytes: usize) -> i64 {
    let path = self.folder.join(format!("seeded-{title}-{number}.mp4"));
    let content: Vec<u8> = (0..bytes).map(|at| (at % 251) as u8).collect();
    std::fs::write(&path, content).unwrap();
    sqlx::query_scalar(
      "INSERT INTO episodes (title_id, kind, season, number, name, path, file_name, media_type, byte_count, \
       modified_at, duration_ms, creator, created_at, changed_at) \
       VALUES ($1, 'clip', 1, $2, $3, $4, $5, 'video/mp4', $6, 1, $7, '', 1, 1) RETURNING id",
    )
    .bind(title)
    .bind(number)
    .bind(format!("Part {number}"))
    .bind(path.to_string_lossy().as_ref())
    .bind(format!("seeded-{title}-{number}.mp4"))
    .bind(bytes as i64)
    .bind(duration_ms)
    .fetch_one(&self.pool)
    .await
    .unwrap()
  }
}

/// The ids of a page's items.
pub fn ids(page: &Value) -> Vec<i64> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["id"].as_i64().unwrap())
    .collect()
}
