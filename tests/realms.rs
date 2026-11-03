//! The two realms: an account's own titles and playlists read by it alone, the organization's by every account and
//! changed by the administrator and their creator; the service token naming its account; and the guard.

use reqwest::Method;
use serde_json::json;

use crate::support::{CONSOLE, MEMBER_A, MEMBER_B, Service, TOKEN, ids};

#[tokio::test]
async fn every_route_is_guarded() {
  let service = Service::start().await;
  let code = async |bearer: Option<&str>, headers: &[(&str, &str)]| {
    let (status, answer) = service.ask(Method::GET, "/titles", bearer, headers, None).await;
    (status, answer["code"].as_str().unwrap_or("").to_owned())
  };
  assert_eq!(code(None, &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(code(Some("wrong"), &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(code(Some(CONSOLE), &[("Owner-Role", "member")]).await, (403, "forbidden".to_owned()));
  assert_eq!(code(Some(CONSOLE), &[("Owner-Key", "2")]).await, (403, "forbidden".to_owned()));
  assert_eq!(
    code(Some(CONSOLE), &[("Owner-Key", "2"), ("Owner-Role", "guest")]).await,
    (403, "forbidden".to_owned())
  );
  let long = "k".repeat(201);
  assert_eq!(
    code(Some(CONSOLE), &[("Owner-Key", &long), ("Owner-Role", "member")]).await,
    (403, "forbidden".to_owned())
  );
  assert_eq!(code(Some(CONSOLE), &[("Owner-Key", "2"), ("Owner-Role", "member")]).await.0, 200);
  assert_eq!(code(Some(TOKEN), &[]).await.0, 200);
  let (status, refusal) = service.service(Method::GET, "/nowhere/at/all", None).await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("no-route")));
  service.stop().await;
}

#[tokio::test]
async fn an_accounts_own_titles_are_its_alone() {
  let service = Service::start().await;
  let (status, mine) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/titles",
      Some(json!({ "realm": "mine", "name": "Home Videos" })),
    )
    .await;
  assert_eq!(status, 201, "{mine}");
  let mine = mine["id"].as_i64().unwrap();
  let (_, shared) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/titles",
      Some(json!({ "realm": "organization", "name": "Team Clips" })),
    )
    .await;
  let shared = shared["id"].as_i64().unwrap();

  let (_, a) = service.member(MEMBER_A, Method::GET, "/titles", None).await;
  assert_eq!(ids(&a), [mine, shared]);
  let (_, b) = service.member(MEMBER_B, Method::GET, "/titles", None).await;
  assert_eq!(ids(&b), [shared]);
  let (_, b_mine) = service.member(MEMBER_B, Method::GET, "/titles?realm=mine", None).await;
  assert!(ids(&b_mine).is_empty());
  let (status, hidden) = service.member(MEMBER_B, Method::GET, &format!("/titles/{mine}"), None).await;
  assert_eq!((status, hidden["code"].as_str()), (404, Some("not-found")));

  let (status, refused) = service
    .member(MEMBER_B, Method::PATCH, &format!("/titles/{shared}"), Some(json!({ "name": "Taken" })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")));
  let (status, renamed) = service
    .admin(Method::PATCH, &format!("/titles/{shared}"), Some(json!({ "name": "Team Reels" })))
    .await;
  assert_eq!((status, renamed["name"].as_str()), (200, Some("Team Reels")));
  let (status, _) = service
    .member(
      MEMBER_A,
      Method::PATCH,
      &format!("/titles/{shared}"),
      Some(json!({ "description": "ours" })),
    )
    .await;
  assert_eq!(status, 200);

  let (status, refused) = service.service(Method::GET, "/titles?realm=mine", None).await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-account")));
  let (_, organization) = service.service(Method::GET, "/titles", None).await;
  assert_eq!(ids(&organization), [shared]);
  let (_, named) = service
    .ask(Method::GET, "/titles?realm=mine", Some(TOKEN), &[("Owner-Key", MEMBER_A)], None)
    .await;
  assert_eq!(ids(&named), [mine]);
  let (status, _) = service.service(Method::GET, &format!("/titles/{mine}"), None).await;
  assert_eq!(status, 200, "the service token reads every member by id");

  let (_, found) = service.member(MEMBER_B, Method::GET, "/titles?q=reel", None).await;
  assert_eq!(ids(&found), [shared]);
  let (status, refused) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/titles",
      Some(json!({ "realm": "mine", "name": "two\nlines" })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (400, Some("bad-request")));
  service.stop().await;
}

#[tokio::test]
async fn playlists_hold_what_their_readers_read_and_answer_each_viewers_progress() {
  let service = Service::start().await;
  let own = service.seed_title("mine", MEMBER_A, "Own").await;
  let team = service.seed_title("organization", "", "Team").await;
  let own_episode = service.seed_episode(own, 1, 60_000, 10).await;
  let first = service.seed_episode(team, 1, 60_000, 10).await;
  let second = service.seed_episode(team, 2, 60_000, 10).await;
  let third = service.seed_episode(team, 3, 60_000, 10).await;

  let (status, refused) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/playlists",
      Some(json!({ "realm": "organization", "name": "Mixed", "episodeIds": [first, own_episode] })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("not-in-realm")));
  let (status, refused) = service
    .member(
      MEMBER_B,
      Method::POST,
      "/playlists",
      Some(json!({ "realm": "mine", "name": "Theirs", "episodeIds": [own_episode] })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("unknown-episode")));
  let (status, mixed) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/playlists",
      Some(json!({ "realm": "mine", "name": "Mixed", "episodeIds": [own_episode, first] })),
    )
    .await;
  assert_eq!(status, 201, "{mixed}");

  let (status, playlist) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/playlists",
      Some(json!({ "realm": "organization", "name": "Run", "episodeIds": [first, third] })),
    )
    .await;
  assert_eq!(status, 201);
  let playlist_id = playlist["id"].as_i64().unwrap();
  let (status, playlist) = service
    .member(
      MEMBER_A,
      Method::POST,
      &format!("/playlists/{playlist_id}/items"),
      Some(json!({ "episodeIds": [second], "at": 1 })),
    )
    .await;
  assert_eq!(status, 201);
  assert_eq!(playlist["episodeIds"], json!([first, second, third]));
  assert_eq!(playlist["durationMs"], 180_000);
  let (status, _) = service
    .member(
      MEMBER_B,
      Method::PATCH,
      &format!("/playlists/{playlist_id}"),
      Some(json!({ "name": "Mine now" })),
    )
    .await;
  assert_eq!(status, 403);

  let progress = format!("/playlists/{playlist_id}/progress");
  let (_, fresh) = service.member(MEMBER_A, Method::GET, &progress, None).await;
  assert_eq!((fresh["next"].as_i64(), fresh["doneCount"].as_i64()), (Some(first), Some(0)));

  let (status, position) = service
    .member(
      MEMBER_A,
      Method::PUT,
      &format!("/episodes/{first}/position"),
      Some(json!({ "seconds": 55.0 })),
    )
    .await;
  assert_eq!((status, position["done"].as_bool()), (200, Some(true)));
  let (_, position) = service
    .member(
      MEMBER_A,
      Method::PUT,
      &format!("/episodes/{first}/position"),
      Some(json!({ "seconds": 3.0 })),
    )
    .await;
  assert_eq!(position["done"], true, "a watched episode stays watched");
  let (_, after_first) = service.member(MEMBER_A, Method::GET, &progress, None).await;
  assert_eq!(
    (after_first["next"].as_i64(), after_first["nextSeconds"].as_f64()),
    (Some(second), Some(0.0))
  );

  service
    .member(
      MEMBER_A,
      Method::PUT,
      &format!("/episodes/{second}/position"),
      Some(json!({ "seconds": 20.5 })),
    )
    .await;
  let (_, resumed) = service.member(MEMBER_A, Method::GET, &progress, None).await;
  assert_eq!((resumed["next"].as_i64(), resumed["nextSeconds"].as_f64()), (Some(second), Some(20.5)));
  assert_eq!(resumed["items"][1]["seconds"], 20.5);

  let (_, other) = service.member(MEMBER_B, Method::GET, &progress, None).await;
  assert_eq!(
    (other["next"].as_i64(), other["doneCount"].as_i64()),
    (Some(first), Some(0)),
    "positions are each viewer's"
  );
  let (_, episode) = service.member(MEMBER_B, Method::GET, &format!("/episodes/{first}"), None).await;
  assert!(episode["position"].is_null());
  let (_, episode) = service.member(MEMBER_A, Method::GET, &format!("/episodes/{first}"), None).await;
  assert_eq!(episode["position"]["done"], true);

  let (status, refused) = service
    .service(Method::PUT, &format!("/episodes/{first}/position"), Some(json!({ "seconds": 1 })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-account")));
  let (_, title_progress) =
    service.member(MEMBER_A, Method::GET, &format!("/titles/{team}/progress"), None).await;
  assert_eq!(title_progress["next"].as_i64(), Some(second));
  service.stop().await;
}
