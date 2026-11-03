//! The OpenAPI document: the committed one is what the code generates, it states exactly the routes and the tool
//! entries docs/api.md lists, and each entry's input is the schema the tool's folder places.

use std::collections::BTreeSet;

use bmo_theater::openapi::{openapi, openapi_json};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The routes docs/api.md states, by method and path.
const ROUTES: [&str; 55] = [
  "get /health",
  "get /libraries",
  "post /libraries",
  "get /libraries/{libraryId}",
  "patch /libraries/{libraryId}",
  "delete /libraries/{libraryId}",
  "post /libraries/{libraryId}/scans",
  "get /libraries/{libraryId}/scans",
  "get /titles",
  "post /titles",
  "get /titles/{titleId}",
  "patch /titles/{titleId}",
  "delete /titles/{titleId}",
  "get /titles/{titleId}/poster",
  "get /titles/{titleId}/episodes",
  "post /titles/{titleId}/episodes",
  "get /titles/{titleId}/progress",
  "get /episodes/{episodeId}",
  "patch /episodes/{episodeId}",
  "delete /episodes/{episodeId}",
  "get /episodes/{episodeId}/stream",
  "get /episodes/{episodeId}/poster",
  "get /episodes/{episodeId}/position",
  "put /episodes/{episodeId}/position",
  "get /episodes/{episodeId}/subtitles",
  "post /episodes/{episodeId}/subtitles",
  "post /episodes/{episodeId}/burns",
  "get /episodes/{episodeId}/burns",
  "get /burns/{burnId}",
  "get /subtitles/{subtitleId}",
  "delete /subtitles/{subtitleId}",
  "get /subtitles/{subtitleId}.vtt",
  "get /subtitles/{subtitleId}/original",
  "get /playlists",
  "post /playlists",
  "get /playlists/{playlistId}",
  "patch /playlists/{playlistId}",
  "delete /playlists/{playlistId}",
  "post /playlists/{playlistId}/items",
  "get /playlists/{playlistId}/progress",
  "get /shares",
  "post /shares",
  "get /shares/{token}",
  "delete /shares/{token}",
  "get /shares/{token}/head",
  "get /shares/{token}/playlist",
  "get /shares/{token}/episodes",
  "get /shares/{token}/episodes/{episodeId}/stream",
  "get /shares/{token}/episodes/{episodeId}/poster",
  "get /shares/{token}/episodes/{episodeId}/subtitles",
  "get /shares/{token}/subtitles/{subtitleId}.vtt",
  "get /events",
  "get /console-manifest",
  "get /console/words",
  "get /console/{file}",
];

#[test]
fn the_committed_document_is_the_generated_one() {
  let committed = std::fs::read_to_string(format!("{ROOT}/docs/openapi.json")).unwrap_or_default();
  assert!(
    committed == openapi_json(),
    "docs/openapi.json differs from what the code generates; write it again with `bmo-theater openapi`"
  );
}

#[test]
fn the_document_states_every_route_and_tool_and_nothing_else() {
  let document = openapi();
  let stated: BTreeSet<String> = document["paths"]
    .as_object()
    .unwrap()
    .iter()
    .flat_map(|(path, methods)| {
      methods.as_object().unwrap().keys().map(move |method| format!("{method} {path}"))
    })
    .collect();
  let listed: BTreeSet<String> = ROUTES.iter().map(|route| (*route).to_owned()).collect();
  assert_eq!(stated, listed);
  assert_eq!(document["info"]["version"], env!("CARGO_PKG_VERSION"));
  assert_eq!(document["x-events"].as_array().unwrap().len(), bmo_theater::events::KINDS.len());

  let manifest: toml::Table = std::fs::read_to_string(format!("{ROOT}/tools/theater/tool.toml"))
    .unwrap()
    .parse()
    .unwrap();
  let entries = manifest["entry"].as_array().unwrap();
  let tools = document["x-tools"].as_array().unwrap();
  assert_eq!(tools.len(), entries.len());
  for (tool, entry) in tools.iter().zip(entries) {
    assert_eq!(tool["name"].as_str(), entry["name"].as_str());
    assert_eq!(tool["namespace"].as_str(), entry["namespace"].as_str());
    let (method, path) = tool["route"].as_str().unwrap().split_once(' ').unwrap();
    assert!(
      listed.contains(&format!("{} {path}", method.to_lowercase())),
      "{} calls no route",
      tool["name"]
    );
    let file = entry["input"].as_str().unwrap();
    let placed: serde_json::Value =
      serde_json::from_str(&std::fs::read_to_string(format!("{ROOT}/tools/theater/{file}")).unwrap())
        .unwrap();
    assert_eq!(tool["input"], placed, "{file}");
  }
}
