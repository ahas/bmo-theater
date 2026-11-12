//! Share links: made by a reader of the member, one live link to each, read with the console's credential alone,
//! opening its episode or its playlist's and nothing else, streamed with ranges, ended by time and by deletion, and
//! given a head only where a share address is set.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{MEMBER_A, MEMBER_B, Service};

#[tokio::test]
async fn a_link_opens_what_it_names_and_nothing_else() {
  let service = Service::start_with(Some(json!({ "en": { "theater": { "title": "Theater", "share": { "locale": "en_US", "siteName": "The Theater" } } } }))).await;
  let team = service.seed_title("organization", "", "Team").await;
  let own = service.seed_title("mine", MEMBER_A, "Own").await;
  let episode = service.seed_episode(team, 1, 60_000, 1000).await;
  let other = service.seed_episode(team, 2, 60_000, 1000).await;
  let private = service.seed_episode(own, 1, 60_000, 1000).await;

  let (status, refused) = service
    .member(MEMBER_B, Method::POST, "/shares", Some(json!({ "episodeId": private })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (404, Some("not-found")));
  let (status, refused) = service
    .member(
      MEMBER_B,
      Method::POST,
      "/shares",
      Some(json!({ "episodeId": episode, "playlistId": 1 })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (400, Some("bad-request")));
  let (status, link) = service
    .member(MEMBER_B, Method::POST, "/shares", Some(json!({ "episodeId": episode })))
    .await;
  assert_eq!(status, 201, "{link}");
  let token = link["token"].as_str().unwrap().to_owned();
  assert_eq!(token.len(), 32);
  assert_eq!(link["url"], format!("https://console.test/theater/share/{token}"));
  let (status, again) = service
    .member(MEMBER_A, Method::POST, "/shares", Some(json!({ "episodeId": episode })))
    .await;
  assert_eq!((status, again["token"].as_str()), (200, Some(token.as_str())));

  let read = service.shared(&format!("/shares/{token}"), &[]).await;
  assert_eq!(read.status().as_u16(), 200);
  let episodes: Value =
    service.shared(&format!("/shares/{token}/episodes"), &[]).await.json().await.unwrap();
  assert_eq!(episodes["items"].as_array().unwrap().len(), 1);
  assert!(episodes["items"][0]["position"].is_null());
  let stretch = service
    .shared(&format!("/shares/{token}/episodes/{episode}/stream"), &[("Range", "bytes=10-19")])
    .await;
  assert_eq!(stretch.status().as_u16(), 206);
  assert_eq!(stretch.headers()["content-range"], "bytes 10-19/1000");
  let past = service
    .shared(&format!("/shares/{token}/episodes/{episode}/stream"), &[("Range", "bytes=5000-")])
    .await;
  assert_eq!(past.status().as_u16(), 416);
  let elsewhere = service.shared(&format!("/shares/{token}/episodes/{other}/stream"), &[]).await;
  assert_eq!(elsewhere.status().as_u16(), 404);
  let playlist = service.shared(&format!("/shares/{token}/playlist"), &[]).await;
  assert_eq!(playlist.status().as_u16(), 404);
  let nobody = service.raw(Method::GET, &format!("/shares/{token}"), None, &[], None).await;
  assert_eq!(nobody.status().as_u16(), 401);
  let head: Value = service
    .shared(&format!("/shares/{token}/head?language=en"), &[])
    .await
    .json()
    .await
    .unwrap();
  assert_eq!(head["title"], "Team · Part 1");
  assert_eq!(head["type"], "video.episode");
  assert_eq!(head["siteName"], "The Theater");
  assert_eq!(head["url"], link["url"]);

  let (_, listed) = service
    .member(MEMBER_A, Method::GET, &format!("/shares?episodeId={episode}"), None)
    .await;
  assert_eq!(listed["items"].as_array().unwrap().len(), 1);
  let (status, _) = service.member(MEMBER_A, Method::DELETE, &format!("/shares/{token}"), None).await;
  assert_eq!(status, 403, "only its creator and the administrator end a link");
  let (status, _) = service.member(MEMBER_B, Method::DELETE, &format!("/shares/{token}"), None).await;
  assert_eq!(status, 204);
  assert_eq!(service.shared(&format!("/shares/{token}"), &[]).await.status().as_u16(), 404);
  service.stop().await;
}

#[tokio::test]
async fn a_link_with_no_share_address_has_no_head() {
  let service = Service::start_tuned(None, |settings| settings.share.address = String::new()).await;
  let team = service.seed_title("organization", "", "Team").await;
  let episode = service.seed_episode(team, 1, 60_000, 1000).await;
  let (status, link) = service
    .member(MEMBER_A, Method::POST, "/shares", Some(json!({ "episodeId": episode })))
    .await;
  assert_eq!((status, &link["url"]), (201, &Value::Null), "{link}");
  let token = link["token"].as_str().unwrap();
  let head = service.shared(&format!("/shares/{token}/head"), &[]).await;
  assert_eq!(head.status().as_u16(), 404);
  service.stop().await;
}

#[tokio::test]
async fn a_playlist_link_opens_its_episodes_and_an_ended_one_is_gone() {
  let service = Service::start().await;
  let team = service.seed_title("organization", "", "Team").await;
  let first = service.seed_episode(team, 1, 60_000, 100).await;
  let second = service.seed_episode(team, 2, 60_000, 100).await;
  let (_, playlist) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/playlists",
      Some(json!({ "realm": "organization", "name": "Run", "episodeIds": [second, first] })),
    )
    .await;
  let playlist_id = playlist["id"].as_i64().unwrap();
  let (status, link) = service
    .member(MEMBER_A, Method::POST, "/shares", Some(json!({ "playlistId": playlist_id })))
    .await;
  assert_eq!(status, 201);
  let token = link["token"].as_str().unwrap();
  let episodes: Value =
    service.shared(&format!("/shares/{token}/episodes"), &[]).await.json().await.unwrap();
  let order: Vec<i64> = episodes["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["id"].as_i64().unwrap())
    .collect();
  assert_eq!(order, [second, first]);
  let shared: Value = service.shared(&format!("/shares/{token}/playlist"), &[]).await.json().await.unwrap();
  assert_eq!(shared["name"], "Run");
  let stream = service.shared(&format!("/shares/{token}/episodes/{first}/stream"), &[]).await;
  assert_eq!(stream.status().as_u16(), 200);

  let (status, refused) = service
    .member(MEMBER_A, Method::POST, "/shares", Some(json!({ "episodeId": first, "expiresAt": 1 })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (400, Some("bad-request")));
  let soon = bmo_theater::wire::now_ms() + 300;
  let (status, short) = service
    .member(
      MEMBER_A,
      Method::POST,
      "/shares",
      Some(json!({ "episodeId": first, "expiresAt": soon })),
    )
    .await;
  assert_eq!(status, 201);
  let short = short["token"].as_str().unwrap().to_owned();
  tokio::time::sleep(std::time::Duration::from_millis(400)).await;
  assert_eq!(service.shared(&format!("/shares/{short}"), &[]).await.status().as_u16(), 410);
  assert_eq!(service.shared(&format!("/shares/{short}"), &[]).await.status().as_u16(), 404);
  service.stop().await;
}
