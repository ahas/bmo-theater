//! The titles in each order a list takes: by id, the latest added first, by name with its letter case folded,
//! and the one watched last first, each paged on after a title; and an order it does not know refused.

use reqwest::Method;
use serde_json::Value;

use crate::support::{MEMBER_A, Service, ids};

/// A title of `owner`'s own realm seeded straight into the tables, made at `created_at`.
async fn title_made_at(service: &Service, owner: &str, name: &str, created_at: i64) -> i64 {
  sqlx::query_scalar(
    "INSERT INTO titles (realm, owner, creator, name, kind, created_at, changed_at) \
     VALUES ('mine', $1, $1, $2, 'show', $3, $3) RETURNING id",
  )
  .bind(owner)
  .bind(name)
  .bind(created_at)
  .fetch_one(&service.pool)
  .await
  .unwrap()
}

async fn listed(service: &Service, query: &str) -> Value {
  let (status, page) = service.member(MEMBER_A, Method::GET, &format!("/titles?{query}"), None).await;
  assert_eq!(status, 200, "{query}: {page}");
  page
}

#[tokio::test]
async fn titles_stand_in_each_order_and_page_on_after_one() {
  let service = Service::start().await;
  let banana = title_made_at(&service, MEMBER_A, "banana", 100).await;
  let apple = title_made_at(&service, MEMBER_A, "Apple", 200).await;
  let cherry = title_made_at(&service, MEMBER_A, "cherry", 300).await;
  let eclair = title_made_at(&service, MEMBER_A, "\u{e9}clair", 150).await;
  let watched = service.seed_episode(banana, 1, 60_000, 10).await;
  let added = service.seed_episode(eclair, 1, 60_000, 10).await;
  sqlx::query("UPDATE episodes SET created_at = 500 WHERE id = $1")
    .bind(added)
    .execute(&service.pool)
    .await
    .unwrap();
  for (episode, at) in [(watched, 1_000), (added, 900)] {
    sqlx::query(
      "INSERT INTO positions (account, episode_id, seconds, done, updated_at) VALUES ($1, $2, 5, FALSE, $3)",
    )
    .bind(MEMBER_A)
    .bind(episode)
    .bind(at)
    .execute(&service.pool)
    .await
    .unwrap();
  }

  assert_eq!(ids(&listed(&service, "").await), [banana, apple, cherry, eclair]);
  let by_id = listed(&service, "order=id").await;
  assert_eq!(ids(&by_id), [banana, apple, cherry, eclair]);
  assert_eq!(by_id["items"][3]["addedAt"], 500);
  assert_eq!(by_id["items"][2]["addedAt"], 300);
  assert_eq!(by_id["items"][2]["posterWidth"], Value::Null);

  let first = listed(&service, "order=added&limit=2").await;
  assert_eq!((ids(&first), first["more"].as_bool()), (vec![eclair, cherry], Some(true)));
  let rest = listed(&service, &format!("order=added&limit=2&after={cherry}")).await;
  assert_eq!((ids(&rest), rest["more"].as_bool()), (vec![apple, banana], Some(false)));

  let first = listed(&service, "order=name&limit=3").await;
  assert_eq!((ids(&first), first["more"].as_bool()), (vec![apple, banana, cherry], Some(true)));
  let rest = listed(&service, &format!("order=name&after={cherry}")).await;
  assert_eq!((ids(&rest), rest["more"].as_bool()), (vec![eclair], Some(false)));

  let first = listed(&service, "order=watched&limit=1").await;
  assert_eq!((ids(&first), first["more"].as_bool()), (vec![banana], Some(true)));
  let rest = listed(&service, &format!("order=watched&after={banana}")).await;
  assert_eq!((ids(&rest), rest["more"].as_bool()), (vec![eclair, cherry, apple], Some(false)));
  let rest = listed(&service, &format!("order=watched&limit=1&after={cherry}")).await;
  assert_eq!(ids(&rest), [apple]);

  // The service token naming no account reads the organization's titles, ordered as `added`.
  let organization = sqlx::query_scalar::<_, i64>(
    "INSERT INTO titles (realm, owner, creator, name, kind, created_at, changed_at) \
     VALUES ('organization', '', '', 'Team', 'clip', 50, 50) RETURNING id",
  )
  .fetch_one(&service.pool)
  .await
  .unwrap();
  let (status, unnamed) = service.service(Method::GET, "/titles?order=watched", None).await;
  assert_eq!((status, ids(&unnamed)), (200, vec![organization]));

  let (status, refused) = service.member(MEMBER_A, Method::GET, "/titles?order=newest", None).await;
  assert_eq!((status, refused["code"].as_str()), (400, Some("bad-request")));
  service.stop().await;
}
