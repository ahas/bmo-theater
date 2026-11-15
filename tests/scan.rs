//! A library read into titles: the plan its names make, and a scan of a folder of tiny videos made with ffmpeg,
//! read again after a file goes and another comes, streamed with ranges, given subtitles and burned; and the size
//! of each poster read as it is taken, and as the service starts for one kept before.

use reqwest::Method;
use serde_json::{Value, json};

use bmo_theater::scan::{plan, root_name};
use bmo_theater::wire::TitleKind;

use crate::support::{Service, ffmpeg_here, make_video, scratch};

fn files(names: &[&str]) -> Vec<String> {
  names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn folders_and_names_plan_the_titles() {
  let files = files(&[
    "S01/Show - S01E01 Pilot [KR] [1080p].mp4",
    "S01/Show - S01E02 Second [KR] [1080p].mp4",
    "S02/Show - S02E01 Return [KR] [1080p].mp4",
    "Far Lands/Show - Far Lands E01 Robot [KR].mp4",
    "Spin Off/S1/Spin Off - S01E01 Start.mp4",
    "Spin Off/Spin Off - Trailer.mp4",
    "Films/One Film (2019).mkv",
    "Films/Another_Film.mp4",
  ]);
  assert_eq!(root_name(&files, "media"), "Show");
  let planned = plan(&files, "Show");
  let by_key = |key: &str| planned.iter().find(|title| title.key == key).unwrap_or_else(|| panic!("{key}"));

  let show = by_key("folder:");
  assert_eq!((show.name.as_str(), show.kind), ("Show", TitleKind::Show));
  let numbers: Vec<_> = show.episodes.iter().map(|episode| (episode.season, episode.number)).collect();
  assert_eq!(numbers, [(Some(1), Some(1)), (Some(1), Some(2)), (Some(2), Some(1))]);
  assert_eq!(show.episodes[0].name, "Pilot");

  let far = by_key("folder:Far Lands");
  assert_eq!((far.name.as_str(), far.kind), ("Far Lands", TitleKind::Show));
  assert_eq!((far.episodes[0].season, far.episodes[0].number), (None, Some(1)));

  let spin = by_key("folder:Spin Off");
  assert_eq!(spin.episodes.len(), 2);
  let trailer = spin.episodes.iter().find(|episode| episode.number.is_none()).unwrap();
  assert_eq!((trailer.season, trailer.name.as_str()), (None, "Spin Off - Trailer"));

  let film = by_key("file:Films/One Film (2019).mkv");
  assert_eq!((film.name.as_str(), film.kind), ("One Film (2019)", TitleKind::Film));
  assert_eq!(by_key("file:Films/Another_Film.mp4").name, "Another Film");
  assert_eq!(planned.len(), 5);
  assert_eq!(root_name(&files[5..], "media"), "media");
}

async fn scan_ended(service: &Service, library: i64) -> Value {
  let mut last = Value::Null;
  service
    .until(async |service| {
      let (_, scans) = service.admin(Method::GET, &format!("/libraries/{library}/scans"), None).await;
      last = scans["items"][0].clone();
      last["state"] != "running"
    })
    .await;
  last
}

#[tokio::test]
async fn a_library_is_scanned_streamed_rescanned_and_burned() {
  if !ffmpeg_here() {
    eprintln!("skipped: ffmpeg or ffprobe is not installed, so no test video can be made or read");
    return;
  }
  let library = scratch();
  make_video(&library.join("S01/Show - S01E01 Pilot [KR].mp4"));
  make_video(&library.join("S01/Show - S01E02 Second [KR].mp4"));
  make_video(&library.join("Films/A Film.mp4"));
  std::fs::write(library.join("notes.htm"), "not a video").unwrap();
  let service = Service::start().await;
  let path = library.to_string_lossy().into_owned();

  let (status, refused) = service
    .member("2", Method::POST, "/libraries", Some(json!({ "path": path, "realm": "mine" })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")));
  let (status, refused) = service
    .admin(Method::POST, "/libraries", Some(json!({ "path": "relative/folder", "realm": "mine" })))
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("not-a-folder")));

  let (status, made) = service
    .admin(Method::POST, "/libraries", Some(json!({ "path": path, "realm": "organization" })))
    .await;
  assert_eq!(status, 201, "{made}");
  assert_eq!(made["name"], "Show");
  let id = made["id"].as_i64().unwrap();
  let (status, twice) = service
    .admin(Method::POST, "/libraries", Some(json!({ "path": path, "realm": "organization" })))
    .await;
  assert_eq!((status, twice["code"].as_str()), (409, Some("conflict")));
  let scan = scan_ended(&service, id).await;
  assert_eq!(scan["state"], "finished", "{scan}");
  assert_eq!((scan["foundCount"].as_i64(), scan["addedCount"].as_i64()), (Some(3), Some(3)));

  let (_, titles) = service.member("3", Method::GET, &format!("/titles?libraryId={id}"), None).await;
  let titles = titles["items"].as_array().unwrap().clone();
  assert_eq!(titles.len(), 2, "{titles:?}");
  let show = titles.iter().find(|title| title["kind"] == "show").unwrap();
  assert_eq!((show["name"].as_str(), show["episodeCount"].as_i64()), (Some("Show"), Some(2)));
  assert_eq!(show["hasPoster"], true);
  assert_eq!((show["posterWidth"].as_i64(), show["posterHeight"].as_i64()), (Some(640), Some(480)));
  let show_id = show["id"].as_i64().unwrap();
  let (_, episodes) = service.member("3", Method::GET, &format!("/titles/{show_id}/episodes"), None).await;
  let first = episodes["items"][0].clone();
  assert_eq!((first["season"].as_i64(), first["number"].as_i64()), (Some(1), Some(1)));
  assert_eq!(first["name"], "Pilot");
  assert_eq!(
    (first["videoCodec"].as_str(), first["audioCodec"].as_str()),
    (Some("h264"), Some("aac"))
  );
  assert_eq!((first["width"].as_i64(), first["height"].as_i64()), (Some(160), Some(120)));
  assert_eq!((first["posterWidth"].as_i64(), first["posterHeight"].as_i64()), (Some(640), Some(480)));
  let duration = first["durationMs"].as_i64().unwrap();
  assert!((900..=1200).contains(&duration), "{duration}");
  let episode = first["id"].as_i64().unwrap();

  let headers = [("Owner-Key", "3"), ("Owner-Role", "member"), ("Range", "bytes=0-99")];
  let stretch = service
    .raw(
      Method::GET,
      &format!("/episodes/{episode}/stream"),
      Some(crate::support::CONSOLE),
      &headers,
      None,
    )
    .await;
  assert_eq!(stretch.status().as_u16(), 206);
  assert_eq!(stretch.headers()["content-type"], "video/mp4");
  assert_eq!(stretch.bytes().await.unwrap().len(), 100);
  let headers = [("Owner-Key", "3"), ("Owner-Role", "member"), ("Range", "bytes=99999999-")];
  let past = service
    .raw(
      Method::GET,
      &format!("/episodes/{episode}/stream"),
      Some(crate::support::CONSOLE),
      &headers,
      None,
    )
    .await;
  assert_eq!(past.status().as_u16(), 416);
  let poster = service
    .raw(
      Method::GET,
      &format!("/episodes/{episode}/poster"),
      Some(crate::support::CONSOLE),
      &[("Owner-Key", "3"), ("Owner-Role", "member")],
      None,
    )
    .await;
  assert_eq!(poster.headers()["content-type"], "image/jpeg");

  // One file goes and another comes; the next scan marks the one and adds the other, and deletes nothing.
  std::fs::remove_file(library.join("S01/Show - S01E02 Second [KR].mp4")).unwrap();
  make_video(&library.join("S02/Show - S02E01 Return.mp4"));
  let (status, _) = service.admin(Method::POST, &format!("/libraries/{id}/scans"), None).await;
  assert_eq!(status, 201);
  let scan = scan_ended(&service, id).await;
  assert_eq!(
    (scan["addedCount"].as_i64(), scan["missingCount"].as_i64()),
    (Some(1), Some(1)),
    "{scan}"
  );
  let (_, episodes) = service.admin(Method::GET, &format!("/titles/{show_id}/episodes"), None).await;
  let items = episodes["items"].as_array().unwrap();
  assert_eq!(items.len(), 3);
  let gone = items.iter().find(|item| item["name"] == "Second").unwrap();
  assert_eq!(gone["available"], false);

  // A subtitle added and burned in: the burn files a clip under the same title.
  let srt = library.join("pilot.srt");
  std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,900\nHello\n").unwrap();
  let (status, subtitle) = service
    .service(
      Method::POST,
      &format!("/episodes/{episode}/subtitles"),
      Some(json!({ "path": srt.to_string_lossy(), "language": "en" })),
    )
    .await;
  assert_eq!(status, 201, "{subtitle}");
  assert_eq!(subtitle["cueCount"], 1);
  let subtitle_id = subtitle["id"].as_i64().unwrap();
  let (status, vtt) = service.service(Method::GET, &format!("/subtitles/{subtitle_id}.vtt"), None).await;
  assert_eq!((status, vtt.as_str()), (200, Some("WEBVTT\n\n00:00:00.000 --> 00:00:00.900\nHello\n")));
  let (status, burn) = service
    .admin(
      Method::POST,
      &format!("/episodes/{episode}/burns"),
      Some(json!({ "subtitleId": subtitle_id })),
    )
    .await;
  assert_eq!(status, 201, "{burn}");
  let burn_id = burn["id"].as_i64().unwrap();
  let mut ended = Value::Null;
  service
    .until(async |service| {
      ended = service.admin(Method::GET, &format!("/burns/{burn_id}"), None).await.1;
      ended["state"] != "running"
    })
    .await;
  assert_eq!(ended["state"], "finished", "{ended}");
  let clip = ended["clipId"].as_i64().unwrap();
  let (_, clip) = service.admin(Method::GET, &format!("/episodes/{clip}"), None).await;
  assert_eq!((clip["kind"].as_str(), clip["titleId"].as_i64()), (Some("clip"), Some(show_id)));
  assert_eq!(clip["burnedFrom"], json!({ "episodeId": episode, "subtitleId": subtitle_id }));
  assert_eq!(clip["label"], "burned: en");
  assert_eq!(clip["videoCodec"], "h264");
  assert!(clip["byteCount"].as_i64().unwrap() > 0);

  // A library's episode is not deleted alone; the library's deletion leaves its files.
  let (status, refused) = service.admin(Method::DELETE, &format!("/episodes/{episode}"), None).await;
  assert_eq!((status, refused["code"].as_str()), (409, Some("conflict")));
  let (status, _) = service.admin(Method::DELETE, &format!("/libraries/{id}"), None).await;
  assert_eq!(status, 204);
  assert!(library.join("S01/Show - S01E01 Pilot [KR].mp4").exists());
  let (_, titles) = service.admin(Method::GET, "/titles", None).await;
  assert!(titles["items"].as_array().unwrap().is_empty(), "{titles}");

  service.stop().await;
  let _ = std::fs::remove_dir_all(library);
}

#[tokio::test]
async fn a_video_is_filed_by_upload_and_by_path() {
  if !ffmpeg_here() {
    eprintln!("skipped: ffmpeg or ffprobe is not installed, so no test video can be made or read");
    return;
  }
  let folder = scratch();
  let video = folder.join("made.mp4");
  make_video(&video);
  let service = Service::start().await;
  let (_, title) = service
    .member("2", Method::POST, "/titles", Some(json!({ "realm": "mine", "name": "Reports" })))
    .await;
  assert_eq!(title["kind"], "clip");
  let title = title["id"].as_i64().unwrap();

  let boundary = "theater-test-boundary";
  let mut form = Vec::new();
  form.extend(
    format!(
      "--{boundary}\r\nContent-Disposition: form-data; name=\"label\"\r\n\r\nweek one\r\n--{boundary}\r\n\
     Content-Disposition: form-data; name=\"file\"; filename=\"Weekly Report.mp4\"\r\n\
     Content-Type: video/mp4\r\n\r\n"
    )
    .as_bytes(),
  );
  form.extend(std::fs::read(&video).unwrap());
  form.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
  let response = reqwest::Client::new()
    .post(format!("{}/titles/{title}/episodes", service.address))
    .bearer_auth(crate::support::CONSOLE)
    .header("Owner-Key", "2")
    .header("Owner-Role", "member")
    .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
    .body(form)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status().as_u16(), 201);
  let filed: Value = response.json().await.unwrap();
  assert_eq!(
    (filed["name"].as_str(), filed["label"].as_str()),
    (Some("Weekly Report"), Some("week one"))
  );
  assert_eq!(filed["kind"], "clip");
  assert_eq!(filed["hasPoster"], true);
  let id = filed["id"].as_i64().unwrap();
  assert!(service.folder.join(format!("root/episodes/{id}/Weekly Report.mp4")).exists());

  let (status, refused) = service
    .member(
      "2",
      Method::POST,
      &format!("/titles/{title}/episodes"),
      Some(json!({ "path": video.to_string_lossy() })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (403, Some("forbidden")));
  let headers = [("Owner-Key", "2")];
  let (status, by_path) = service
    .ask(
      Method::POST,
      &format!("/titles/{title}/episodes"),
      Some(crate::support::TOKEN),
      &headers,
      Some(json!({ "path": video.to_string_lossy(), "name": "Made" })),
    )
    .await;
  assert_eq!(status, 201, "{by_path}");
  assert!(video.exists(), "a file filed by path is copied, not moved");
  let text = folder.join("notes.txt");
  std::fs::write(&text, "words").unwrap();
  let (status, refused) = service
    .ask(
      Method::POST,
      &format!("/titles/{title}/episodes"),
      Some(crate::support::TOKEN),
      &headers,
      Some(json!({ "path": text.to_string_lossy() })),
    )
    .await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("not-a-video")));

  let (status, _) = service.member("2", Method::DELETE, &format!("/episodes/{id}"), None).await;
  assert_eq!(status, 204);
  assert!(!service.folder.join(format!("root/episodes/{id}")).exists());
  service.stop().await;
  let _ = std::fs::remove_dir_all(folder);
}

#[tokio::test]
async fn a_posters_size_is_read_as_it_is_taken_and_as_the_service_starts() {
  if !ffmpeg_here() {
    eprintln!("skipped: ffmpeg or ffprobe is not installed, so no test video can be made or read");
    return;
  }
  let folder = scratch();
  let video = folder.join("made.mp4");
  make_video(&video);
  let mut service = Service::start().await;
  // A poster that does not read stands first, and is passed over.
  let unread = service.seed_title("organization", "", "Unread").await;
  let broken = service.seed_episode(unread, 1, 60_000, 10).await;
  sqlx::query("UPDATE episodes SET has_poster = TRUE WHERE id = $1")
    .bind(broken)
    .execute(&service.pool)
    .await
    .unwrap();
  let (_, title) = service
    .member("2", Method::POST, "/titles", Some(json!({ "realm": "mine", "name": "Reports" })))
    .await;
  let title = title["id"].as_i64().unwrap();
  let headers = [("Owner-Key", "2")];
  let (status, filed) = service
    .ask(
      Method::POST,
      &format!("/titles/{title}/episodes"),
      Some(crate::support::TOKEN),
      &headers,
      Some(json!({ "path": video.to_string_lossy() })),
    )
    .await;
  assert_eq!(status, 201, "{filed}");
  assert_eq!((filed["posterWidth"].as_i64(), filed["posterHeight"].as_i64()), (Some(640), Some(480)));
  let id = filed["id"].as_i64().unwrap();
  let (_, read) = service.member("2", Method::GET, &format!("/titles/{title}"), None).await;
  assert_eq!((read["posterWidth"].as_i64(), read["posterHeight"].as_i64()), (Some(640), Some(480)));

  sqlx::query("UPDATE episodes SET poster_width = NULL, poster_height = NULL")
    .execute(&service.pool)
    .await
    .unwrap();
  let (_, forgotten) = service.member("2", Method::GET, &format!("/episodes/{id}"), None).await;
  assert_eq!(forgotten["posterWidth"], Value::Null);
  service.restart().await;
  service
    .until(async |service| {
      let (_, episode) = service.member("2", Method::GET, &format!("/episodes/{id}"), None).await;
      episode["posterWidth"].as_i64() == Some(640) && episode["posterHeight"].as_i64() == Some(480)
    })
    .await;
  let (_, broken) = service.service(Method::GET, &format!("/episodes/{broken}"), None).await;
  assert_eq!((broken["hasPoster"].as_bool(), broken["posterWidth"].as_i64()), (Some(true), None));
  service.stop().await;
  let _ = std::fs::remove_dir_all(folder);
}
