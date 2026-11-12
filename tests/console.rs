//! The console bundle: none answered where none is built, and a built one answered by its manifest, its module,
//! its stylesheet and its words to the console's credential alone, the service token refused.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{CONSOLE, Service, TOKEN};

#[tokio::test]
async fn no_bundle_is_answered_where_none_is_built() {
  let service = Service::start().await;
  for path in ["/console-manifest", "/console/words", "/console/bundle-0000000000000000.js"] {
    let (status, refusal) = service.ask(Method::GET, path, Some(CONSOLE), &[], None).await;
    assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")), "{path}");
  }
  service.stop().await;
}

#[tokio::test]
async fn a_built_bundle_is_answered_by_its_manifest() {
  let words = json!({ "en": { "theater": { "title": "Theater" } }, "ko": { "theater": { "title": "T" } } });
  let service = Service::start_with(Some(words.clone())).await;
  let (status, manifest) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200);
  assert_eq!(manifest["name"], "theater");
  assert_eq!(manifest["title"], "theater.title");
  assert_eq!(manifest["icon"], "i-material-symbols-theaters-outline");
  assert_eq!(manifest["screenPath"], "/theater");
  assert_eq!(
    (manifest["role"].as_str(), manifest["ownerKey"].as_bool()),
    (Some("member"), Some(true))
  );
  assert_eq!(
    (manifest["events"].as_bool(), manifest["websocket"].as_bool()),
    (Some(true), Some(false))
  );
  assert_eq!(manifest["pushKinds"], json!([]));
  assert_eq!(manifest["calls"], json!([]));
  assert_eq!(manifest["shareHead"], "/shares/{token}/head");
  let reads: Vec<&str> = manifest["shareReads"]
    .as_array()
    .unwrap()
    .iter()
    .map(|read| read.as_str().unwrap())
    .collect();
  assert!(reads.contains(&"/shares/{token}/episodes/{episodeId}/stream"));
  for read in &reads {
    assert_eq!(read.matches("{token}").count(), 1, "{read}");
  }

  let module = manifest["bundle"].as_str().unwrap();
  assert!(module.starts_with("/console/bundle-") && module.ends_with(".js"));
  let answered = service.raw(Method::GET, module, Some(CONSOLE), &[], None).await;
  assert_eq!(answered.headers()["content-type"], "text/javascript");
  assert_eq!(answered.headers()["cache-control"], "private, max-age=31536000, immutable");
  let (_, served): (u16, Value) =
    service.ask(Method::GET, "/console/words", Some(CONSOLE), &[], None).await;
  assert_eq!(served, words);
  for bearer in [None, Some(TOKEN)] {
    for path in ["/console-manifest", "/console/words", module] {
      let (status, _) = service.ask(Method::GET, path, bearer, &[], None).await;
      assert_eq!(status, 401, "{path} by {bearer:?}");
    }
  }
  service.stop().await;
}

#[tokio::test]
async fn a_reader_is_given_its_own_events_and_the_organizations() {
  let service = Service::start().await;
  service
    .member("2", Method::POST, "/titles", Some(json!({ "realm": "mine", "name": "Private" })))
    .await;
  service
    .member("2", Method::POST, "/titles", Some(json!({ "realm": "organization", "name": "Shared" })))
    .await;
  let page = async |key: &str| {
    let headers = [("Owner-Key", key), ("Owner-Role", "member"), ("Accept", "application/json")];
    let (_, page) = service
      .ask(Method::GET, "/events?kinds=title.created", Some(CONSOLE), &headers, None)
      .await;
    page["items"]
      .as_array()
      .unwrap()
      .iter()
      .map(|event| event["data"]["name"].as_str().unwrap().to_owned())
      .collect::<Vec<_>>()
  };
  assert_eq!(page("2").await, ["Private", "Shared"]);
  assert_eq!(page("3").await, ["Shared"]);
  service.stop().await;
}
