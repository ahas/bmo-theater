//! The service as it runs: its settings, its database, its file root, the events it publishes and the console
//! face it serves.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bmo_console_face::ConsoleFace;
use serde::Serialize;
use sqlx::PgPool;

use crate::caller::Standing;
use crate::events::Events;
use crate::settings::Settings;
use crate::wire::Realm;

/// The service's state, shared by every route and task.
pub struct App {
  pub settings: Settings,
  pub pool: PgPool,
  pub events: Events,
  /// The console face, where one is served (`docs/api.md`, 「Console」).
  pub face: Option<ConsoleFace>,
}

impl App {
  pub fn new(settings: Settings, pool: PgPool, face: Option<ConsoleFace>) -> Arc<Self> {
    Arc::new(Self {
      settings,
      pool,
      events: Events::new(),
      face,
    })
  }

  /// The file root.
  pub fn root(&self) -> &Path {
    &self.settings.files.root
  }

  /// Where an episode's poster frame stands.
  pub fn poster_path(&self, episode: i64) -> PathBuf {
    self.root().join("posters").join(format!("{episode}.jpg"))
  }

  /// The folder a filed or burned episode's file stands in.
  pub fn episode_folder(&self, episode: i64) -> PathBuf {
    self.root().join("episodes").join(episode.to_string())
  }

  /// The folder a subtitle's original and WebVTT stand in.
  pub fn subtitle_folder(&self, subtitle: i64) -> PathBuf {
    self.root().join("subtitles").join(subtitle.to_string())
  }

  /// A subtitle's WebVTT.
  pub fn vtt_path(&self, subtitle: i64) -> PathBuf {
    self.subtitle_folder(subtitle).join("subtitle.vtt")
  }

  /// The folder a burn works in while it runs.
  pub fn burn_folder(&self, burn: i64) -> PathBuf {
    self.root().join("burns").join(burn.to_string())
  }

  /// The folder uploads arrive in.
  pub fn uploads(&self) -> PathBuf {
    self.root().join("uploads")
  }

  /// Why the file root cannot be written, or none.
  pub fn unready(&self) -> Option<String> {
    let probe = self.uploads().join(".ready");
    match std::fs::create_dir_all(self.uploads()).and_then(|()| std::fs::write(&probe, b"")) {
      Ok(()) => {
        let _ = std::fs::remove_file(probe);
        None
      }
      Err(why) => Some(format!("the file root {} cannot be written: {why}", self.root().display())),
    }
  }

  /// One event published, for the account owning the member where it stands in `mine`.
  pub fn publish(&self, kind: &str, data: impl Serialize, standing: &Standing) {
    let account = (standing.realm == Realm::Mine).then(|| standing.owner.clone());
    self.events.publish(kind, data, account);
  }

  /// One event published for one account alone.
  pub fn publish_for(&self, kind: &str, data: impl Serialize, account: &str) {
    self.events.publish(kind, data, Some(account.to_owned()));
  }
}

/// A file removed, where it stands; a failure noted and passed over, since what it held is let go of.
pub async fn remove_file(path: &Path) {
  if let Err(why) = tokio::fs::remove_file(path).await
    && why.kind() != std::io::ErrorKind::NotFound
  {
    eprintln!("bmo-theater: {} could not be removed: {why}", path.display());
  }
}

/// A folder removed with what it holds, where it stands.
pub async fn remove_folder(path: &Path) {
  if let Err(why) = tokio::fs::remove_dir_all(path).await
    && why.kind() != std::io::ErrorKind::NotFound
  {
    eprintln!("bmo-theater: {} could not be removed: {why}", path.display());
  }
}
