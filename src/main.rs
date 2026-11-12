//! The `bmo-theater` command: `serve` answers the routes until it is told to stop; `migrate` brings the database
//! to its schema; `health` ends 0 only where the service its settings name answers ready; `openapi` prints the
//! OpenAPI document; `console-reach` prints how a console reaches the service; `tool-env` writes the env file the
//! tool reads the service's address, token and account from.

use std::path::PathBuf;
use std::process::ExitCode;

use bmo_theater::openapi::openapi_json;
use bmo_theater::settings::Settings;
use bmo_theater::tool_env::{Written, write_env};
use bmo_theater::wire::Health;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-theater", version, about = "The theater service and its command.")]
struct Command {
  #[command(subcommand)]
  verb: Verb,
}

#[derive(Subcommand)]
enum Verb {
  /// Brings the database to its schema and answers the routes until the process is told to stop.
  Serve,
  /// Brings the database the settings name to the schema the binary carries.
  Migrate,
  /// Whether the service the settings name answers ready: exit code 0 where it does, 1 where not.
  Health,
  /// Prints the OpenAPI document.
  Openapi,
  /// Prints how a console reaches the service, as one JSON object, for a deployment's console probe.
  ConsoleReach,
  /// Writes the env file the tool reads the service's address, token and account from, under the data root.
  ToolEnv {
    /// The data root; the settings' where none is given.
    #[arg(long, value_name = "dir")]
    data_root: Option<PathBuf>,
  },
}

async fn health(settings: &Settings) -> Result<(), String> {
  let base = settings.base_address();
  let response = reqwest::Client::builder()
    .no_proxy()
    .build()
    .map_err(|why| format!("no HTTP client: {why}"))?
    .get(format!("{base}/health"))
    .bearer_auth(&settings.server.token)
    .send()
    .await
    .map_err(|why| format!("{base}: {why}"))?;
  let status = response.status();
  let health: Health = response.json().await.map_err(|why| format!("{base} answered {status}: {why}"))?;
  if health.ready {
    Ok(())
  } else {
    Err(format!(
      "the service is not ready: {}",
      health.reason.as_deref().unwrap_or("it gave no reason")
    ))
  }
}

#[tokio::main]
async fn main() -> ExitCode {
  let Command { verb } = Command::parse();
  let ended = match verb {
    Verb::Openapi => {
      print!("{}", openapi_json());
      Ok(())
    }
    verb => match Settings::load() {
      Err(why) => Err(why),
      Ok(settings) => match verb {
        Verb::Serve => bmo_theater::serve(settings).await,
        Verb::Migrate => match bmo_theater::db::connect(&settings.database.url).await {
          Ok(pool) => bmo_theater::db::migrate(&pool).await,
          Err(why) => Err(why),
        },
        Verb::Health => health(&settings).await,
        Verb::ConsoleReach => {
          println!("{}", bmo_theater::console::reach(&settings));
          Ok(())
        }
        Verb::ToolEnv { data_root } => {
          let root = data_root.unwrap_or_else(|| settings.data_root());
          write_env(&settings, &root).map(|written| match written {
            Written::Written(path) => println!("written {}", path.display()),
            Written::Unchanged(path) => println!("unchanged {}", path.display()),
          })
        }
        Verb::Openapi => Ok(()),
      },
    },
  };
  match ended {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo-theater: {why}");
      ExitCode::FAILURE
    }
  }
}
