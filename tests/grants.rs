//! Grants: an episode shared with an account is watched by that account and nothing more, a stranger is
//! answered as before, an ended grant shuts the episode again, a link is untouched by either, and grants are
//! made by whoever changes the episode.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{CONSOLE, MEMBER_A, MEMBER_B, Service};

/// An account the episode is never granted to.
const STRANGER: &str = "4";

/// A subtitle of two cues added to an episode by an account, as an upload: its id.
async fn add_subtitle(service: &Service, account: &str, episode: i64) -> i64 {
  let boundary = "grant-boundary";
  let form = format!(
    "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"en.srt\"\r\n\
     Content-Type: application/x-subrip\r\n\r\n1\r\n00:00:01,000 --> 00:00:02,000\r\nHello\r\n\r\n\
     2\r\n00:00:03,000 --> 00:00:04,000\r\nBye\r\n\r\n--{boundary}--\r\n"
  );
  let response = reqwest::Client::new()
    .post(format!("{}/episodes/{episode}/subtitles", service.address))
    .bearer_auth(CONSOLE)
    .header("Owner-Key", account)
    .header("Owner-Role", "member")
    .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
    .body(form)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status().as_u16(), 201);
  let added: Value = response.json().await.unwrap();
  added["id"].as_i64().unwrap()
}

/// The statuses an account is answered on the watching reads of an episode and one of its subtitles.
async fn watching(service: &Service, account: &str, episode: i64, subtitle: i64) -> [u16; 5] {
  let mut statuses = [0; 5];
  let reads = [
    format!("/episodes/{episode}"),
    format!("/episodes/{episode}/stream"),
    format!("/episodes/{episode}/subtitles"),
    format!("/subtitles/{subtitle}.vtt"),
    format!("/subtitles/{subtitle}/original"),
  ];
  for (status, path) in statuses.iter_mut().zip(&reads) {
    *status = service.member(account, Method::GET, path, None).await.0;
  }
  statuses
}

#[tokio::test]
async fn a_granted_episode_is_watched_by_its_account_alone_until_the_grant_ends() {
  let service = Service::start().await;
  let own = service.seed_title("mine", MEMBER_A, "Own").await;
  let episode = service.seed_episode(own, 1, 60_000, 1000).await;
  let subtitle = add_subtitle(&service, MEMBER_A, episode).await;
  let (status, link) = service
    .member(MEMBER_A, Method::POST, "/shares", Some(json!({ "episodeId": episode })))
    .await;
  assert_eq!(status, 201, "{link}");
  let token = link["token"].as_str().unwrap().to_owned();
  assert_eq!(watching(&service, MEMBER_B, episode, subtitle).await, [404; 5]);

  let (status, grants) = service
    .member(
      MEMBER_A,
      Method::POST,
      &format!("/episodes/{episode}/grants"),
      Some(json!({ "users": [MEMBER_B, MEMBER_A] })),
    )
    .await;
  assert_eq!(status, 200, "{grants}");
  let items = grants["items"].as_array().unwrap();
  assert_eq!(items.len(), 1, "the owner is passed over: {grants}");
  assert_eq!(
    (items[0]["user"].as_str(), items[0]["permission"].as_str(), items[0]["grantor"].as_str()),
    (Some(MEMBER_B), Some("read"), Some(MEMBER_A))
  );

  assert_eq!(watching(&service, MEMBER_B, episode, subtitle).await, [200; 5]);
  let (_, read) = service.member(MEMBER_B, Method::GET, &format!("/episodes/{episode}"), None).await;
  assert_eq!(read["granted"], true);
  let (_, read) = service.member(MEMBER_A, Method::GET, &format!("/episodes/{episode}"), None).await;
  assert_eq!(read["granted"], false);
  let ranged = service
    .raw(
      Method::GET,
      &format!("/episodes/{episode}/stream"),
      Some(CONSOLE),
      &[("Owner-Key", MEMBER_B), ("Owner-Role", "member"), ("Range", "bytes=0-9")],
      None,
    )
    .await;
  assert_eq!(ranged.status().as_u16(), 206);
  let (status, position) = service
    .member(
      MEMBER_B,
      Method::PUT,
      &format!("/episodes/{episode}/position"),
      Some(json!({ "seconds": 12.0 })),
    )
    .await;
  assert_eq!((status, position["seconds"].as_f64()), (200, Some(12.0)));
  let (_, own_read) = service.member(MEMBER_A, Method::GET, &format!("/episodes/{episode}"), None).await;
  assert_eq!(own_read["position"], Value::Null, "a position stays its viewer's own");

  assert_eq!(watching(&service, STRANGER, episode, subtitle).await, [404; 5]);

  let refused = [
    (Method::PATCH, format!("/episodes/{episode}"), Some(json!({ "name": "Taken" }))),
    (Method::DELETE, format!("/episodes/{episode}"), None),
    (Method::POST, "/shares".to_owned(), Some(json!({ "episodeId": episode }))),
    (
      Method::POST,
      format!("/episodes/{episode}/burns"),
      Some(json!({ "subtitleId": subtitle })),
    ),
    (Method::GET, format!("/episodes/{episode}/burns"), None),
    (Method::DELETE, format!("/subtitles/{subtitle}"), None),
    (Method::GET, format!("/titles/{own}"), None),
    (Method::GET, format!("/titles/{own}/episodes"), None),
    (Method::GET, format!("/episodes/{episode}/grants"), None),
    (
      Method::POST,
      format!("/episodes/{episode}/grants"),
      Some(json!({ "users": [STRANGER] })),
    ),
  ];
  for (method, path, body) in refused {
    let (status, answer) = service.member(MEMBER_B, method.clone(), &path, body).await;
    assert_eq!(status, 404, "{method} {path} opens to a grant: {answer}");
  }
  let (status, titles) = service.member(MEMBER_B, Method::GET, "/titles", None).await;
  assert_eq!((status, titles["items"].as_array().map(Vec::len)), (200, Some(0)));

  let (status, changed) = service
    .member(
      MEMBER_A,
      Method::PUT,
      &format!("/episodes/{episode}/grants/{MEMBER_B}"),
      Some(json!({ "permission": "edit" })),
    )
    .await;
  assert_eq!((status, changed["permission"].as_str()), (200, Some("edit")));
  let (status, _) = service
    .member(
      MEMBER_B,
      Method::PATCH,
      &format!("/episodes/{episode}"),
      Some(json!({ "name": "Taken" })),
    )
    .await;
  assert_eq!(status, 404, "edit opens no write");

  let path = format!("/episodes/{episode}/grants/{MEMBER_B}");
  assert_eq!(service.member(MEMBER_A, Method::DELETE, &path, None).await.0, 204);
  let (status, again) = service.member(MEMBER_A, Method::DELETE, &path, None).await;
  assert_eq!((status, again["code"].as_str()), (404, Some("not-found")));
  assert_eq!(watching(&service, MEMBER_B, episode, subtitle).await, [404; 5]);

  let stream = service.shared(&format!("/shares/{token}/episodes/{episode}/stream"), &[]).await;
  assert_eq!(stream.status().as_u16(), 200, "the link opens as it did through the grant's life");
  service.stop().await;
}

#[tokio::test]
async fn grants_are_made_by_whoever_changes_the_episode_and_named_by_account_keys() {
  let service = Service::start().await;
  let team = service.seed_title("organization", "", "Team").await;
  sqlx::query("UPDATE titles SET creator = $1 WHERE id = $2")
    .bind(MEMBER_A)
    .bind(team)
    .execute(&service.pool)
    .await
    .unwrap();
  let episode = service.seed_episode(team, 1, 60_000, 1000).await;
  let grants = format!("/episodes/{episode}/grants");

  let (status, refused) = service
    .member(MEMBER_B, Method::POST, &grants, Some(json!({ "users": [STRANGER] })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")));
  let (status, refused) = service.member(MEMBER_B, Method::GET, &grants, None).await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")));

  let too_long = "k".repeat(201);
  for body in [
    json!({ "users": [] }),
    json!({ "users": ["  "] }),
    json!({ "users": [too_long] }),
    json!({ "users": [STRANGER], "permission": "owner" }),
  ] {
    let (status, refused) = service.member(MEMBER_A, Method::POST, &grants, Some(body.clone())).await;
    assert_eq!((status, refused["code"].as_str()), (400, Some("bad-request")), "{body}");
  }

  let (status, made) = service
    .admin(Method::POST, &grants, Some(json!({ "users": [STRANGER], "permission": "edit" })))
    .await;
  assert_eq!((status, made["items"][0]["permission"].as_str()), (200, Some("edit")));
  let (status, made) = service
    .member(MEMBER_A, Method::POST, &grants, Some(json!({ "users": [STRANGER] })))
    .await;
  assert_eq!(status, 200);
  assert_eq!(
    (made["items"].as_array().map(Vec::len), made["items"][0]["permission"].as_str()),
    (Some(1), Some("read")),
    "granting again sets the permission"
  );
  let (status, missing) = service
    .member(
      MEMBER_A,
      Method::PUT,
      &format!("{grants}/{MEMBER_B}"),
      Some(json!({ "permission": "edit" })),
    )
    .await;
  assert_eq!((status, missing["code"].as_str()), (404, Some("not-found")));

  assert_eq!(
    service.member(MEMBER_A, Method::DELETE, &format!("/episodes/{episode}"), None).await.0,
    204
  );
  let left: i64 = sqlx::query_scalar("SELECT count(*) FROM grants")
    .fetch_one(&service.pool)
    .await
    .unwrap();
  assert_eq!(left, 0, "a grant ends with its episode");
  service.stop().await;
}
