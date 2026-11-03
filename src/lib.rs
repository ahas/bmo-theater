//! The theater service: titles of episodes, filed outputs and libraries indexed in place, streamed with
//! subtitles, playlists, watch positions and share links, and the console bundle that shows them
//! (`docs/design.md`, `docs/api.md`).

pub mod api;
pub mod app;
pub mod burns;
pub mod caller;
pub mod console;
pub mod db;
pub mod error;
pub mod events;
pub mod media;
pub mod names;
pub mod openapi;
pub mod progress;
pub mod range;
pub mod scan;
pub mod settings;
pub mod store;
pub mod subtitles;
pub mod tool_env;
pub mod wire;

use tokio::net::TcpListener;

use crate::app::App;
use crate::console::Bundle;
use crate::settings::Settings;
use crate::wire::now_ms;

/// Brings the database to its schema, and answers the routes on the settings' address until the process is told
/// to stop, the open event streams ended at once.
pub async fn serve(settings: Settings) -> Result<(), String> {
  let listener = TcpListener::bind(settings.server.listen)
    .await
    .map_err(|why| format!("{} could not be listened on: {why}", settings.server.listen))?;
  serve_on(settings, listener, stopped()).await
}

/// The service on a listener already bound, until `stop` resolves.
pub async fn serve_on(
  settings: Settings,
  listener: TcpListener,
  stop: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
  if settings.files.root.as_os_str().is_empty() {
    return Err("no file root is named: set [files] root".to_owned());
  }
  let pool = db::connect(&settings.database.url).await?;
  db::migrate(&pool).await?;
  let cut_short = "the service stopped while it ran";
  for table in ["scans", "burns"] {
    sqlx::query(&format!(
      "UPDATE {table} SET state = 'failed', error = $1, ended_at = $2 WHERE state = 'running'"
    ))
    .bind(cut_short)
    .bind(now_ms())
    .execute(&pool)
    .await
    .map_err(|why| format!("the {table} cut short could not be failed: {why}"))?;
  }
  let bundle = match Bundle::read(&settings.console.folder) {
    Ok(bundle) => Some(bundle),
    Err(why) => {
      eprintln!("bmo-theater: the console bundle is not served: {why}");
      None
    }
  };
  let app = App::new(settings, pool, bundle);
  if let Some(why) = app.unready() {
    eprintln!("bmo-theater: {why}");
  }
  let stopping = std::sync::Arc::clone(&app);
  axum::serve(listener, api::router(app))
    .with_graceful_shutdown(async move {
      stop.await;
      stopping.events.stop();
    })
    .await
    .map_err(|why| why.to_string())
}

/// Resolves once the process is told to stop: an interrupt, or the service manager's terminate.
async fn stopped() {
  let interrupt = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending().await,
    }
  };
  tokio::select! {
    () = interrupt => {}
    () = terminate => {}
  }
}
