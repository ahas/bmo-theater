//! A file served with HTTP ranges: the stretch a header asks for, 200 whole, 206 a stretch, 416 past the end,
//! 304 for the current tag, and a tag that no longer holds answering the whole file.

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use bmo_theater::range::{Wanted, serve, wanted};
use futures_util::StreamExt;

#[test]
fn the_stretch_a_header_asks_for() {
  assert_eq!(wanted(None, 100), Wanted::Whole);
  assert_eq!(wanted(Some("bytes=0-9"), 100), Wanted::Stretch(0, 9));
  assert_eq!(wanted(Some("bytes=90-"), 100), Wanted::Stretch(90, 99));
  assert_eq!(wanted(Some("bytes=-10"), 100), Wanted::Stretch(90, 99));
  assert_eq!(wanted(Some("bytes=-500"), 100), Wanted::Stretch(0, 99));
  assert_eq!(wanted(Some("bytes=50-500"), 100), Wanted::Stretch(50, 99));
  assert_eq!(wanted(Some("bytes=0-1, 5-6"), 100), Wanted::Stretch(0, 1));
  assert_eq!(wanted(Some("bytes=100-"), 100), Wanted::Unsatisfiable);
  assert_eq!(wanted(Some("bytes=-0"), 100), Wanted::Unsatisfiable);
  assert_eq!(wanted(Some("bytes=9-3"), 100), Wanted::Whole);
  assert_eq!(wanted(Some("items=0-3"), 100), Wanted::Whole);
}

async fn body_of(response: axum::response::Response) -> Vec<u8> {
  let mut bytes = Vec::new();
  let mut stream = response.into_body().into_data_stream();
  while let Some(chunk) = stream.next().await {
    bytes.extend(chunk.unwrap());
  }
  bytes
}

#[tokio::test]
async fn a_file_is_answered_whole_by_stretches_and_by_its_tag() {
  let folder = crate::support::scratch();
  let path = folder.join("clip.mp4");
  let content: Vec<u8> = (0..1000u32).map(|at| (at % 251) as u8).collect();
  std::fs::write(&path, &content).unwrap();
  let ask = async |pairs: &[(header::HeaderName, &str)]| {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
      headers.insert(name.clone(), HeaderValue::from_str(value).unwrap());
    }
    serve(&path, "video/mp4", &headers).await.unwrap().unwrap()
  };

  let whole = ask(&[]).await;
  assert_eq!(whole.status(), StatusCode::OK);
  assert_eq!(whole.headers()[header::CONTENT_TYPE], "video/mp4");
  assert_eq!(whole.headers()[header::ACCEPT_RANGES], "bytes");
  let tag = whole.headers()[header::ETAG].to_str().unwrap().to_owned();
  assert_eq!(body_of(whole).await, content);

  let stretch = ask(&[(header::RANGE, "bytes=100-199")]).await;
  assert_eq!(stretch.status(), StatusCode::PARTIAL_CONTENT);
  assert_eq!(stretch.headers()[header::CONTENT_RANGE], "bytes 100-199/1000");
  assert_eq!(stretch.headers()[header::CONTENT_LENGTH], "100");
  assert_eq!(body_of(stretch).await, content[100..200]);

  let past = ask(&[(header::RANGE, "bytes=1000-")]).await;
  assert_eq!(past.status(), StatusCode::RANGE_NOT_SATISFIABLE);
  assert_eq!(past.headers()[header::CONTENT_RANGE], "bytes */1000");

  let held = ask(&[(header::IF_NONE_MATCH, &tag)]).await;
  assert_eq!(held.status(), StatusCode::NOT_MODIFIED);

  let stale = ask(&[(header::RANGE, "bytes=0-9"), (header::IF_RANGE, "\"old\"")]).await;
  assert_eq!(stale.status(), StatusCode::OK);
  let current = ask(&[(header::RANGE, "bytes=0-9"), (header::IF_RANGE, &tag)]).await;
  assert_eq!(current.status(), StatusCode::PARTIAL_CONTENT);

  assert!(serve(&folder.join("gone.mp4"), "video/mp4", &HeaderMap::new()).await.unwrap().is_none());
  let _ = std::fs::remove_dir_all(folder);
}
