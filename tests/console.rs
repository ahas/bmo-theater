//! The console face (docs/api.md, 「Console」): the manifest the service serves, read through bmo-ui's own reading of
//! the contract on the release the client vendors, the face wired to the service's declaration, bundle folder and
//! credential, and no face where no bundle is built. The face's own rules are bmo-console-face's tests'.

use bmo_console_face::read_manifest;
use reqwest::Method;
use serde_json::json;

use crate::support::{CONSOLE, MEMBER_A, Service, TOKEN, release_layer, vendored_release};

#[tokio::test]
async fn the_served_manifest_is_read_by_the_contract_on_the_vendored_release() {
  let words = json!({ "en": { "theater": { "title": "Theater" } }, "ko": { "theater": { "title": "T" } } });
  let service = Service::start_with(Some(words.clone())).await;
  let (status, served) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200, "{served}");
  let read = read_manifest(&served, &vendored_release()).unwrap_or_else(|why| panic!("{why}"));
  assert_eq!(read["layer"], release_layer());
  assert_eq!(read["name"], "theater");
  assert_eq!(read["screenPath"], "/theater");
  assert_eq!(read["role"], "member");
  assert_eq!(read["ownerKey"], true);
  assert_eq!(read["shareHead"], "/shares/{token}/head");
  assert_eq!(read["shareReads"].as_array().map(Vec::len), Some(7));

  // The face takes the console's credential, never the service's own token.
  assert_eq!(service.ask(Method::GET, "/console-manifest", Some(TOKEN), &[], None).await.0, 401);

  // The words are the bundle's, and the module is answered with no role, since the manifest names share reads.
  let (status, answered) = service
    .ask(Method::GET, read["words"].as_str().unwrap(), Some(CONSOLE), &[], None)
    .await;
  assert_eq!((status, answered), (200, words));
  let module = service
    .raw(Method::GET, read["bundle"].as_str().unwrap(), Some(CONSOLE), &[], None)
    .await;
  assert_eq!(module.status().as_u16(), 200);
  assert_eq!(module.text().await.unwrap(), "export default [];");
  service.stop().await;
}

#[tokio::test]
async fn no_face_stands_where_no_bundle_is_built_and_the_routes_are_answered() {
  let service = Service::start().await;
  let (status, refusal) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("no-route")));
  let (status, _) = service.member(MEMBER_A, Method::GET, "/titles", None).await;
  assert_eq!(status, 200);
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
