//! A viewer's continue watching and next up: the episodes started and not finished, one set aside leaving until
//! it is played again, each realm read apart; and for each show played within thirty days, the episode to go on
//! with along its main line while it is not started.

use bmo_theater::wire::now_ms;
use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{CONSOLE, MEMBER_A, MEMBER_B, Service, ids};

async fn put(service: &Service, key: &str, episode: i64, seconds: f64) {
  let (status, answer) = service
    .member(
      key,
      Method::PUT,
      &format!("/episodes/{episode}/position"),
      Some(json!({ "seconds": seconds })),
    )
    .await;
  assert_eq!(status, 200, "{answer}");
}

/// The moment the account's position in the episode reads as set.
async fn set_at(service: &Service, key: &str, episode: i64, at: i64) {
  sqlx::query("UPDATE positions SET updated_at = $3 WHERE account = $1 AND episode_id = $2")
    .bind(key)
    .bind(episode)
    .bind(at)
    .execute(&service.pool)
    .await
    .unwrap();
}

async fn page(service: &Service, key: &str, path: &str) -> Value {
  let (status, page) = service.member(key, Method::GET, path, None).await;
  assert_eq!(status, 200, "{path}: {page}");
  page
}

#[tokio::test]
async fn continue_watching_holds_what_was_started_and_not_finished_nor_set_aside() {
  let service = Service::start().await;
  let shared = service.seed_title("organization", "", "Shared").await;
  let started = service.seed_episode(shared, 1, 60_000, 10).await;
  let at_nought = service.seed_episode(shared, 2, 60_000, 10).await;
  let finished = service.seed_episode(shared, 3, 60_000, 10).await;
  let own = service.seed_title("mine", MEMBER_A, "Own").await;
  let own_started = service.seed_episode(own, 1, 60_000, 10).await;

  put(&service, MEMBER_A, started, 10.0).await;
  put(&service, MEMBER_A, at_nought, 0.0).await;
  put(&service, MEMBER_A, finished, 59.0).await;
  put(&service, MEMBER_A, own_started, 5.0).await;
  set_at(&service, MEMBER_A, started, 1_000).await;
  set_at(&service, MEMBER_A, own_started, 2_000).await;

  let watching = page(&service, MEMBER_A, "/watching").await;
  assert_eq!(ids(&watching), [own_started, started]);
  assert_eq!(watching["items"][1]["position"]["seconds"], 10.0);
  assert_eq!(watching["items"][1]["position"]["setAside"], false);
  assert_eq!(ids(&page(&service, MEMBER_A, "/watching?realm=organization").await), [started]);
  assert_eq!(ids(&page(&service, MEMBER_A, "/watching?realm=mine").await), [own_started]);
  let first = page(&service, MEMBER_A, "/watching?limit=1").await;
  assert_eq!((ids(&first), first["more"].as_bool()), (vec![own_started], Some(true)));
  let rest = page(&service, MEMBER_A, &format!("/watching?limit=1&after={own_started}")).await;
  assert_eq!((ids(&rest), rest["more"].as_bool()), (vec![started], Some(false)));
  assert!(ids(&page(&service, MEMBER_B, "/watching").await).is_empty());
  let (status, refused) = service.service(Method::GET, "/watching", None).await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-account")));

  // Set aside, it leaves the list with its position kept, and comes back once the position is set again.
  let (status, _) = service.member(MEMBER_A, Method::DELETE, &format!("/watching/{started}"), None).await;
  assert_eq!(status, 204);
  assert_eq!(ids(&page(&service, MEMBER_A, "/watching").await), [own_started]);
  let position = page(&service, MEMBER_A, &format!("/episodes/{started}/position")).await;
  assert_eq!((position["seconds"].as_f64(), position["setAside"].as_bool()), (Some(10.0), Some(true)));
  let headers = [("Owner-Key", MEMBER_A), ("Owner-Role", "member"), ("Accept", "application/json")];
  let (_, events) = service
    .ask(Method::GET, "/events?kinds=position.changed", Some(CONSOLE), &headers, None)
    .await;
  let last = events["items"].as_array().unwrap().last().unwrap().clone();
  assert_eq!(
    (last["data"]["episodeId"].as_i64(), last["data"]["setAside"].as_bool()),
    (Some(started), Some(true))
  );
  let (status, refused) =
    service.member(MEMBER_B, Method::DELETE, &format!("/watching/{started}"), None).await;
  assert_eq!((status, refused["code"].as_str()), (404, Some("not-found")));
  put(&service, MEMBER_A, started, 12.0).await;
  let position = page(&service, MEMBER_A, &format!("/episodes/{started}/position")).await;
  assert_eq!(position["setAside"], false);
  assert_eq!(ids(&page(&service, MEMBER_A, "/watching").await), [started, own_started]);

  // An episode watched by a grant alone stands in no list of the viewer's.
  let (status, granted) = service
    .member(
      MEMBER_A,
      Method::POST,
      &format!("/episodes/{own_started}/grants"),
      Some(json!({ "users": [MEMBER_B] })),
    )
    .await;
  assert!(status == 200 || status == 201, "{granted}");
  put(&service, MEMBER_B, own_started, 7.0).await;
  assert!(ids(&page(&service, MEMBER_B, "/watching").await).is_empty());
  service.stop().await;
}

#[tokio::test]
async fn next_up_offers_each_recent_shows_episode_to_go_on_with_until_it_is_started() {
  let service = Service::start().await;
  let show = service.seed_title("organization", "", "Show").await;
  let special = service.seed_episode(show, 90, 60_000, 10).await;
  let first = service.seed_episode(show, 1, 60_000, 10).await;
  let second = service.seed_episode(show, 2, 60_000, 10).await;
  let third = service.seed_episode(show, 3, 60_000, 10).await;
  let extra = service.seed_episode(show, 91, 60_000, 10).await;
  sqlx::query("UPDATE episodes SET season = 0, number = 1 WHERE id = $1")
    .bind(special)
    .execute(&service.pool)
    .await
    .unwrap();
  sqlx::query("UPDATE episodes SET number = NULL WHERE id = $1")
    .bind(extra)
    .execute(&service.pool)
    .await
    .unwrap();
  let own = service.seed_title("mine", MEMBER_A, "Own Show").await;
  let own_first = service.seed_episode(own, 1, 60_000, 10).await;
  let own_second = service.seed_episode(own, 2, 60_000, 10).await;

  assert!(ids(&page(&service, MEMBER_A, "/next-up").await).is_empty());
  put(&service, MEMBER_A, first, 60.0).await;
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up").await), [second]);
  let progress = page(&service, MEMBER_A, &format!("/titles/{show}/progress")).await;
  assert_eq!(
    (progress["upNext"].as_i64(), progress["upNextSeconds"].as_f64()),
    (Some(second), Some(0.0))
  );

  // A special watched after leaves the main line where it was.
  put(&service, MEMBER_A, special, 60.0).await;
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up").await), [second]);

  // The show played last stands first; each realm is read apart.
  put(&service, MEMBER_A, own_first, 60.0).await;
  let now = now_ms();
  for (episode, at) in [(first, now - 3_000), (special, now - 2_000), (own_first, now - 1_000)] {
    set_at(&service, MEMBER_A, episode, at).await;
  }
  let offered = page(&service, MEMBER_A, "/next-up").await;
  assert_eq!(ids(&offered), [own_second, second]);
  assert_eq!(offered["items"][0]["titleId"], own);
  let head = page(&service, MEMBER_A, "/next-up?limit=1").await;
  assert_eq!((ids(&head), head["more"].as_bool()), (vec![own_second], Some(true)));
  let rest = page(&service, MEMBER_A, &format!("/next-up?after={own_second}")).await;
  assert_eq!((ids(&rest), rest["more"].as_bool()), (vec![second], Some(false)));
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up?realm=organization").await), [second]);
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up?realm=mine").await), [own_second]);
  assert!(ids(&page(&service, MEMBER_B, "/next-up").await).is_empty());
  let (status, refused) = service.service(Method::GET, "/next-up", None).await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-account")));

  // A show last played more than thirty days ago has left.
  set_at(&service, MEMBER_A, own_first, now - 31 * 24 * 3600 * 1000).await;
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up").await), [second]);
  set_at(&service, MEMBER_A, own_first, now - 29 * 24 * 3600 * 1000).await;
  assert_eq!(ids(&page(&service, MEMBER_A, "/next-up").await), [second, own_second]);
  set_at(&service, MEMBER_A, own_first, now - 31 * 24 * 3600 * 1000).await;

  // Started, it stands in continue watching instead.
  put(&service, MEMBER_A, second, 10.0).await;
  assert!(ids(&page(&service, MEMBER_A, "/next-up").await).is_empty());
  assert_eq!(ids(&page(&service, MEMBER_A, "/watching").await), [second]);

  // A finished main line offers nothing, though the progress goes on to the extra.
  put(&service, MEMBER_A, second, 60.0).await;
  put(&service, MEMBER_A, third, 60.0).await;
  assert!(ids(&page(&service, MEMBER_A, "/next-up").await).is_empty());
  let progress = page(&service, MEMBER_A, &format!("/titles/{show}/progress")).await;
  assert_eq!((progress["next"].as_i64(), progress["upNext"].as_i64()), (Some(extra), None));
  service.stop().await;
}
