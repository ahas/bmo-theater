//! Who calls (`docs/api.md`, 「Callers」, 「Realms」): the service token, or the console with a reader's key and
//! role, and what each may read and change of a member standing in a realm.

use axum::http::{HeaderMap, header};
use sqlx::{Postgres, QueryBuilder};

use crate::error::ApiError;
use crate::settings::Settings;
use crate::wire::Realm;

/// The longest `Owner-Key` taken.
pub const LONGEST_KEY: usize = 200;

/// Who calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
  /// The service token's caller, rather than the console's.
  pub service: bool,
  /// The account the call acts for.
  pub account: Option<String>,
  /// Whether it acts as the administrator.
  pub administrator: bool,
}

/// Where a member stands: its realm, the account owning it in `mine` (empty in `organization`), and who made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
  pub realm: Realm,
  pub owner: String,
  pub creator: String,
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
  headers
    .get(header::AUTHORIZATION)?
    .to_str()
    .ok()?
    .strip_prefix("Bearer ")
    .map(str::trim)
}

/// A header's text, trimmed, where it holds any.
pub fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
  headers.get(name)?.to_str().ok().map(str::trim).filter(|value| !value.is_empty())
}

/// Whether the credential is the service's (true), the console's (false), or neither (401).
pub fn credential(settings: &Settings, headers: &HeaderMap) -> Result<bool, ApiError> {
  let given = bearer(headers).ok_or_else(ApiError::unauthorized)?;
  let server = &settings.server.token;
  let console = &settings.console.token;
  if !server.is_empty() && given == server {
    Ok(true)
  } else if !console.is_empty() && given == console {
    Ok(false)
  } else {
    Err(ApiError::unauthorized())
  }
}

/// The caller of a route the screens use: the service, or the console with a reader's key and role.
pub fn caller(settings: &Settings, headers: &HeaderMap) -> Result<Caller, ApiError> {
  let service = credential(settings, headers)?;
  let key = header_text(headers, "owner-key");
  if key.is_some_and(|key| key.chars().count() > LONGEST_KEY) {
    return Err(ApiError::forbidden("The Owner-Key is longer than 200 characters."));
  }
  if service {
    return Ok(Caller {
      service: true,
      account: key.map(str::to_owned),
      administrator: true,
    });
  }
  let key = key.ok_or_else(|| ApiError::forbidden("The call carries no Owner-Key."))?;
  let administrator = match header_text(headers, "owner-role") {
    Some("administrator") => true,
    Some("member") => false,
    _ => return Err(ApiError::forbidden("The call carries no Owner-Role of administrator or member.")),
  };
  Ok(Caller {
    service: false,
    account: Some(key.to_owned()),
    administrator,
  })
}

impl Caller {
  /// The account the call acts for, or `no-account`.
  pub fn account(&self) -> Result<&str, ApiError> {
    self.account.as_deref().ok_or_else(ApiError::no_account)
  }

  /// What a member it makes records as its creator.
  pub fn creator(&self) -> String {
    self.account.clone().unwrap_or_default()
  }

  /// The owner a member it makes in `realm` takes: its account in `mine`, none in `organization`.
  pub fn owner_in(&self, realm: Realm) -> Result<String, ApiError> {
    match realm {
      Realm::Mine => self.account().map(str::to_owned),
      Realm::Organization => Ok(String::new()),
    }
  }

  /// Whether it reads a member standing so: the service reads every one.
  pub fn reads(&self, standing: &Standing) -> bool {
    self.service
      || standing.realm == Realm::Organization
      || self.account.as_deref() == Some(&standing.owner)
  }

  /// The member read, or `not-found` as if it did not stand.
  pub fn read<T>(&self, found: Option<(T, Standing)>, what: &str) -> Result<(T, Standing), ApiError> {
    match found {
      Some((member, standing)) if self.reads(&standing) => Ok((member, standing)),
      _ => Err(ApiError::not_found(format!("No {what} stands there."))),
    }
  }

  /// Whether it changes a member standing so: in `mine` its owner, in `organization` the administrator and its
  /// creator.
  pub fn changes(&self, standing: &Standing) -> bool {
    self.reads(standing)
      && match standing.realm {
        Realm::Mine => true,
        Realm::Organization => self.administrator || self.account.as_deref() == Some(&standing.creator),
      }
  }

  /// `forbidden` where it may not change a member standing so.
  pub fn may_change(&self, standing: &Standing, what: &str) -> Result<(), ApiError> {
    if self.changes(standing) {
      Ok(())
    } else {
      Err(ApiError::forbidden(format!(
        "Only the administrator and its creator change this {what}."
      )))
    }
  }

  /// `forbidden` where it is not the administrator.
  pub fn may_administer(&self, what: &str) -> Result<(), ApiError> {
    if self.administrator {
      Ok(())
    } else {
      Err(ApiError::forbidden(format!("Only the administrator {what}.")))
    }
  }

  /// The condition a list's rows meet, written after `WHERE` into a query: the realm asked for, where it is
  /// given, and the realms the caller reads. A `mine` list naming no account is `no-account`.
  pub fn push_visible(
    &self,
    query: &mut QueryBuilder<'_, Postgres>,
    alias: &str,
    realm: Option<Realm>,
  ) -> Result<(), ApiError> {
    match (realm, &self.account) {
      (Some(Realm::Mine), None) => return Err(ApiError::no_account()),
      (Some(Realm::Mine), Some(account)) => {
        query
          .push(format!("{alias}.realm = 'mine' AND {alias}.owner = "))
          .push_bind(account.clone());
      }
      (Some(Realm::Organization), _) | (None, None) => {
        query.push(format!("{alias}.realm = 'organization'"));
      }
      (None, Some(account)) => {
        query
          .push(format!(
            "({alias}.realm = 'organization' OR ({alias}.realm = 'mine' AND {alias}.owner = "
          ))
          .push_bind(account.clone())
          .push("))");
      }
    }
    Ok(())
  }
}
