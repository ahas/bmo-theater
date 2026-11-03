//! The tool's manifest and its entries, each called against a stand-in theater service that records what it was
//! asked.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as Route, RawQuery, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::any;
use serde_json::{Value, json};
use theater_tool::{ENTRIES, Handed, Service, Session, answer, episode_line};

const DIR: &str = env!("CARGO_MANIFEST_DIR");

/// One request the stand-in was asked: method, path, query, its Owner-Key and its body.
type Asked = (String, String, String, String, Value);

#[derive(Clone, Default)]
struct Stand {
  asked: Arc<Mutex<Vec<Asked>>>,
}

fn title(id: i64, name: &str) -> Value {
  json!({ "id": id, "name": name, "kind": "clip", "realm": "mine", "episodeCount": 1, "description": "" })
}

fn episode(id: i64) -> Value {
  json!({ "id": id, "name": "Clip", "season": null, "number": null, "durationMs": 61_000, "label": "",
    "available": true, "position": null })
}

async fn answer_for(
  State(stand): State<Stand>,
  method: Method,
  headers: HeaderMap,
  Route(rest): Route<String>,
  RawQuery(query): RawQuery,
  body: Bytes,
) -> Response {
  let query = query.unwrap_or_default();
  let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
  let key = headers.get("owner-key").and_then(|value| value.to_str().ok()).unwrap_or("").to_owned();
  stand.asked.lock().unwrap().push((
    method.to_string(),
    format!("/{rest}"),
    query.clone(),
    key,
    body.clone(),
  ));
  let parts: Vec<&str> = rest.split('/').collect();
  match (method.as_str(), parts.as_slice()) {
    ("GET", ["titles"]) => {
      let items = if query.contains("q=Reports") { vec![title(7, "Reports")] } else { Vec::new() };
      Json(json!({ "items": items, "more": false })).into_response()
    }
    ("POST", ["titles"]) => {
      (StatusCode::CREATED, Json(title(9, body["name"].as_str().unwrap_or("")))).into_response()
    }
    ("GET", ["titles", id]) => Json(title(id.parse().unwrap(), "Reports")).into_response(),
    ("GET", ["titles", _, "episodes"]) => {
      Json(json!({ "items": [episode(3)], "more": false })).into_response()
    }
    ("POST", ["titles", _, "episodes"]) => (StatusCode::CREATED, Json(episode(3))).into_response(),
    ("POST", ["shares"]) => (
      StatusCode::CREATED,
      Json(json!({ "token": "t", "url": "https://console.test/theater/share/t" })),
    )
      .into_response(),
    ("GET", ["episodes", id]) => Json(episode(id.parse().unwrap())).into_response(),
    ("GET", ["episodes", _, "subtitles"]) => Json(
      json!({ "items": [{ "id": 4, "language": "en", "format": "srt", "cueCount": 2 }], "more": false }),
    )
    .into_response(),
    ("POST", ["episodes", _, "subtitles"]) => {
      (StatusCode::CREATED, Json(json!({ "id": 4, "language": "en", "cueCount": 2 }))).into_response()
    }
    ("POST", ["episodes", _, "burns"]) => {
      (StatusCode::CREATED, Json(json!({ "id": 5, "state": "running" }))).into_response()
    }
    ("POST", ["playlists"]) | ("POST", ["playlists", _, "items"]) => (
      StatusCode::CREATED,
      Json(json!({ "id": 6, "name": "Run", "realm": "mine", "episodeIds": [3], "durationMs": 61_000 })),
    )
      .into_response(),
    ("GET", [_, _, "progress"]) => Json(json!({
      "items": [{ "episodeId": 3, "durationMs": 61_000, "seconds": 61.0, "done": true },
                { "episodeId": 8, "durationMs": 61_000, "seconds": 12.0, "done": false }],
      "doneCount": 1, "next": 8, "nextSeconds": 12.0,
    }))
    .into_response(),
    _ => (StatusCode::NOT_FOUND, Json(json!({ "code": "no-route", "error": "No route." }))).into_response(),
  }
}

async fn stand() -> (Stand, Service) {
  let stand = Stand::default();
  let app = Router::new().route("/{*rest}", any(answer_for)).with_state(stand.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = format!("http://{}", listener.local_addr().unwrap());
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
  let service = Service {
    url: address,
    token: "token".into(),
    account: Some("1".into()),
  };
  (stand, service)
}

fn handed(entry: &str, arguments: Value) -> Handed {
  Handed {
    protocol: 1,
    kind: "call".into(),
    entry: format!("bmo/{entry}"),
    arguments,
    session: Session::default(),
  }
}

#[test]
fn the_manifest_declares_every_entry_this_executable_answers() {
  let manifest: toml::Table = std::fs::read_to_string(format!("{DIR}/tool.toml")).unwrap().parse().unwrap();
  let entries: Vec<&str> = manifest["entry"]
    .as_array()
    .unwrap()
    .iter()
    .map(|entry| entry["name"].as_str().unwrap())
    .collect();
  assert_eq!(entries, ENTRIES);
  for entry in manifest["entry"].as_array().unwrap() {
    let input = entry["input"].as_str().unwrap();
    let schema: Value =
      serde_json::from_str(&std::fs::read_to_string(format!("{DIR}/{input}")).unwrap()).unwrap();
    assert_eq!(schema["type"], "object", "{input}");
    assert_eq!(entry["capabilities"].as_array().unwrap()[0].as_str(), Some("room.tools"));
  }
}

#[tokio::test]
async fn filing_finds_or_makes_the_title_and_answers_a_link() {
  let (stand, service) = stand().await;
  let filed = answer(
    &handed("theater_file", json!({ "path": "/tmp/intro.mp4", "title": "Reports", "share": true })),
    &service,
  )
  .await;
  assert!(!filed.is_error, "{}", filed.text());
  assert!(filed.text().contains("https://console.test/theater/share/t"), "{}", filed.text());
  let made =
    answer(&handed("theater_file", json!({ "path": "/tmp/intro.mp4", "title": "Fresh" })), &service).await;
  assert!(!made.is_error, "{}", made.text());
  let asked = stand.asked.lock().unwrap().clone();
  let routes: Vec<String> = asked.iter().map(|(method, path, ..)| format!("{method} {path}")).collect();
  assert_eq!(
    routes,
    [
      "GET /titles",
      "POST /titles/7/episodes",
      "POST /shares",
      "GET /titles",
      "POST /titles",
      "POST /titles/9/episodes"
    ]
  );
  assert!(asked.iter().all(|(.., key, _)| key == "1"), "every call names the owner's account");
  assert_eq!(asked[1].4, json!({ "path": "/tmp/intro.mp4" }));
  assert_eq!(asked[4].4, json!({ "realm": "mine", "name": "Fresh", "kind": "clip" }));
}

#[tokio::test]
async fn every_other_entry_calls_its_route() {
  let (stand, service) = stand().await;
  let calls = [
    ("theater_titles", json!({ "realm": "organization", "query": "Rep" }), "GET /titles"),
    ("theater_show", json!({ "titleId": 7 }), "GET /titles/7/episodes"),
    ("theater_show", json!({ "episodeId": 3 }), "GET /episodes/3/subtitles"),
    (
      "theater_subtitle_add",
      json!({ "episodeId": 3, "path": "/tmp/a.srt", "language": "en" }),
      "POST /episodes/3/subtitles",
    ),
    ("theater_playlist_make", json!({ "name": "Run", "episodeIds": [3] }), "POST /playlists"),
    (
      "theater_playlist_add",
      json!({ "playlistId": 6, "episodeIds": [3], "at": 0 }),
      "POST /playlists/6/items",
    ),
    ("theater_progress", json!({ "playlistId": 6 }), "GET /playlists/6/progress"),
    ("theater_progress", json!({ "titleId": 7 }), "GET /titles/7/progress"),
    ("theater_burn", json!({ "episodeId": 3, "subtitleId": 4 }), "POST /episodes/3/burns"),
  ];
  for (entry, arguments, route) in calls {
    let answered = answer(&handed(entry, arguments), &service).await;
    assert!(!answered.is_error, "{entry}: {}", answered.text());
    let last = stand.asked.lock().unwrap().last().cloned().unwrap();
    assert_eq!(format!("{} {}", last.0, last.1), route, "{entry}");
  }
  let progress = answer(&handed("theater_progress", json!({ "playlistId": 6 })), &service).await;
  assert!(progress.text().contains("Next: episode 8, from 0:12."), "{}", progress.text());
  let asked = stand.asked.lock().unwrap().clone();
  assert!(
    asked[0].2.contains("realm=organization") && asked[0].2.contains("q=Rep"),
    "{}",
    asked[0].2
  );
  let refused = answer(&handed("theater_progress", json!({})), &service).await;
  assert!(refused.is_error);
  let unknown = answer(&handed("theater_nothing", json!({})), &service).await;
  assert!(unknown.is_error);
}

#[test]
fn an_episode_reads_as_one_line() {
  let line =
    episode_line(&json!({ "id": 3, "season": 6, "number": 35, "name": "Finale", "durationMs": 661_000,
    "label": "", "available": true, "position": { "seconds": 30.0, "done": false } }));
  assert_eq!(line, "episode 3 · S06E35 Finale · 11:01 · watched to 0:30");
}
