//! Where the theater's tool finds the service (`docs/api.md`, 「The tool」): the data root's env file, the
//! service's base address, token and the account the tool acts for written into it from the settings, readable by the user alone and
//! rewritten only where it would change.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::settings::Settings;

/// The env file under the data root the tool reads the service's address and token from.
pub const ENV_FILE: &str = "env/theater.env";
/// The variable naming the service's base address.
pub const ADDRESS_VARIABLE: &str = "THEATER_ADDRESS";
/// The variable naming the service token.
pub const TOKEN_VARIABLE: &str = "THEATER_TOKEN";
/// The variable naming the account the tool acts for.
pub const ACCOUNT_VARIABLE: &str = "THEATER_ACCOUNT";

/// What writing the env file came to, and where it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Written {
  Written(PathBuf),
  Unchanged(PathBuf),
}

fn bare(value: &str) -> bool {
  !value.is_empty()
    && !value.chars().any(|character| {
      character.is_whitespace() || matches!(character, '"' | '\'' | '`' | '#' | '$' | '\\')
    })
}

/// The env file's text for these settings, or why a value cannot stand in it.
pub fn env_text(settings: &Settings) -> Result<String, String> {
  let address = settings.base_address();
  let token = settings.server.token.as_str();
  for (name, value) in [(ADDRESS_VARIABLE, address.as_str()), (TOKEN_VARIABLE, token)] {
    if !bare(value) {
      return Err(format!(
        "{name} cannot stand bare in {ENV_FILE}: it is empty or holds whitespace, a quote, '#', '$' or '\\'"
      ));
    }
  }
  let account = settings.tool.account.trim();
  if !account.is_empty() && !bare(account) {
    return Err(format!("{ACCOUNT_VARIABLE} cannot stand bare in {ENV_FILE}"));
  }
  Ok(format!(
    "{ADDRESS_VARIABLE}={address}\n{TOKEN_VARIABLE}={token}\n{ACCOUNT_VARIABLE}={account}\n"
  ))
}

/// The env file written under the data root, where it would change.
pub fn write_env(settings: &Settings, data_root: &Path) -> Result<Written, String> {
  let path = data_root.join(ENV_FILE);
  let text = env_text(settings)?;
  if std::fs::read(&path).is_ok_and(|standing| standing == text.as_bytes()) {
    return Ok(Written::Unchanged(path));
  }
  if let Some(folder) = path.parent() {
    std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
  }
  std::fs::write(&path, text.as_bytes()).map_err(|why| format!("{}: {why}", path.display()))?;
  std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
    .map_err(|why| format!("{}: {why}", path.display()))?;
  Ok(Written::Written(path))
}
