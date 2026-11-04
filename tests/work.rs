//! A work's own acts: renaming it, moving it to another title, deleting it, and who may change or delete it
//! (docs/api.md, 「Episodes」).

use reqwest::Method;
use serde_json::json;

use crate::support::{MEMBER_A, MEMBER_B, Service};

/// A title of the organization's, made by one account.
async fn team_title(service: &Service, creator: &str, name: &str) -> i64 {
  sqlx::query_scalar(
    "INSERT INTO titles (realm, owner, creator, name, kind, created_at, changed_at) \
     VALUES ('organization', '', $1, $2, 'show', 1, 1) RETURNING id",
  )
  .bind(creator)
  .bind(name)
  .fetch_one(&service.pool)
  .await
  .unwrap()
}

/// A library's episode, indexed from a file in the library's folder.
async fn library_episode(service: &Service) -> i64 {
  let library: i64 = sqlx::query_scalar(
    "INSERT INTO libraries (realm, owner, creator, name, path, created_at) \
     VALUES ('organization', '', '', 'Files', '/tmp/theater-library', 1) RETURNING id",
  )
  .fetch_one(&service.pool)
  .await
  .unwrap();
  let title: i64 = sqlx::query_scalar(
    "INSERT INTO titles (realm, owner, creator, name, kind, library_id, created_at, changed_at) \
     VALUES ('organization', '', '', 'Indexed Show', 'show', $1, 1, 1) RETURNING id",
  )
  .bind(library)
  .fetch_one(&service.pool)
  .await
  .unwrap();
  sqlx::query_scalar(
    "INSERT INTO episodes (title_id, kind, library_id, season, number, name, path, file_name, media_type, \
       byte_count, modified_at, duration_ms, creator, created_at, changed_at) \
     VALUES ($1, 'episode', $2, 1, 1, 'Part', '/tmp/theater-library/part.mp4', 'part.mp4', 'video/mp4', 1, 1, \
       60000, '', 1, 1) RETURNING id",
  )
  .bind(title)
  .bind(library)
  .fetch_one(&service.pool)
  .await
  .unwrap()
}

#[tokio::test]
async fn a_work_is_renamed_moved_and_deleted_within_its_realm() {
  let service = Service::start().await;
  let shelf = service.seed_title("mine", MEMBER_A, "Introductions").await;
  let other = service.seed_title("mine", MEMBER_A, "Archive").await;
  let work = service.seed_episode(shelf, 1, 60_000, 32).await;

  let (status, renamed) = service
    .member(
      MEMBER_A,
      Method::PATCH,
      &format!("/episodes/{work}"),
      Some(json!({ "name": "Introduction", "label": "rev.4" })),
    )
    .await;
  assert_eq!(status, 200, "{renamed}");
  assert_eq!(renamed["name"].as_str(), Some("Introduction"));
  assert_eq!(renamed["label"].as_str(), Some("rev.4"));

  let (status, moved) = service
    .member(MEMBER_A, Method::PATCH, &format!("/episodes/{work}"), Some(json!({ "titleId": other })))
    .await;
  assert_eq!(status, 200, "{moved}");
  assert_eq!(moved["titleId"].as_i64(), Some(other));
  assert_eq!(moved["name"].as_str(), Some("Introduction"), "a move keeps the work as it stands");
  let (_, left) = service.member(MEMBER_A, Method::GET, &format!("/titles/{shelf}/episodes"), None).await;
  assert!(left["items"].as_array().unwrap().is_empty());
  let (_, joined) = service.member(MEMBER_A, Method::GET, &format!("/titles/{other}/episodes"), None).await;
  assert_eq!(joined["items"][0]["id"].as_i64(), Some(work));
  let (status, _) = service.member(MEMBER_A, Method::GET, &format!("/episodes/{work}"), None).await;
  assert_eq!(status, 200, "a move deletes nothing");

  let (status, _) = service.member(MEMBER_A, Method::DELETE, &format!("/episodes/{work}"), None).await;
  assert_eq!(status, 204);
  let (status, gone) = service.member(MEMBER_A, Method::GET, &format!("/episodes/{work}"), None).await;
  assert_eq!((status, gone["code"].as_str()), (404, Some("not-found")));
  service.stop().await;
}

#[tokio::test]
async fn a_work_moves_only_within_its_realm_and_never_off_a_library() {
  let service = Service::start().await;
  let mine = service.seed_title("mine", MEMBER_A, "Mine").await;
  let organization = service.seed_title("organization", "", "Organization").await;
  let work = service.seed_episode(mine, 1, 60_000, 8).await;

  let (status, refused) = service
    .member(
      MEMBER_A,
      Method::PATCH,
      &format!("/episodes/{work}"),
      Some(json!({ "titleId": organization })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("not-in-realm")), "{refused}");

  let theirs = service.seed_title("mine", MEMBER_B, "Theirs").await;
  let (status, refused) = service
    .member(
      MEMBER_A,
      Method::PATCH,
      &format!("/episodes/{work}"),
      Some(json!({ "titleId": theirs })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("unknown-title")), "{refused}");

  let indexed = library_episode(&service).await;
  let (status, refused) = service
    .admin(Method::PATCH, &format!("/episodes/{indexed}"), Some(json!({ "titleId": organization })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (409, Some("conflict")), "{refused}");
  let (status, refused) = service.admin(Method::DELETE, &format!("/episodes/{indexed}"), None).await;
  assert_eq!((status, refused["code"].as_str()), (409, Some("conflict")), "{refused}");
  service.stop().await;
}

#[tokio::test]
async fn only_a_title_s_creator_or_the_administrator_changes_its_works() {
  let service = Service::start().await;
  let a_show = team_title(&service, MEMBER_A, "A Show").await;
  let b_show = team_title(&service, MEMBER_B, "B Show").await;
  let work = service.seed_episode(a_show, 1, 60_000, 8).await;

  let (status, refused) = service
    .member(MEMBER_B, Method::PATCH, &format!("/episodes/{work}"), Some(json!({ "name": "Taken" })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")), "{refused}");
  assert_eq!(
    refused["error"].as_str(),
    Some("Only the administrator and its creator change this episode.")
  );

  let (status, refused) = service
    .member(
      MEMBER_A,
      Method::PATCH,
      &format!("/episodes/{work}"),
      Some(json!({ "titleId": b_show })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")), "{refused}");
  assert_eq!(
    refused["error"].as_str(),
    Some("Only the administrator and its creator change this title.")
  );

  let (status, refused) =
    service.member(MEMBER_B, Method::DELETE, &format!("/episodes/{work}"), None).await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")), "{refused}");

  let (status, _) = service
    .admin(Method::PATCH, &format!("/episodes/{work}"), Some(json!({ "titleId": b_show })))
    .await;
  assert_eq!(status, 200);
  let (status, _) = service.admin(Method::DELETE, &format!("/episodes/{work}"), None).await;
  assert_eq!(status, 204);
  service.stop().await;
}
