//! The events the theater publishes (`docs/api.md`, 「Events the theater publishes」): kept since the start, at
//! most the last 1000, numbered upward from the start's moment in milliseconds times 1000, and handed to every
//! open stream as they are published.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Serialize;
use tokio::sync::{broadcast, watch};

use crate::wire::StreamEvent;
use crate::wire::now_ms;

/// How many events are kept.
pub const KEPT: usize = 1000;

/// The kinds the theater publishes.
pub const KINDS: [&str; 23] = [
  "library.created",
  "library.changed",
  "library.deleted",
  "scan.running",
  "scan.finished",
  "scan.failed",
  "title.created",
  "title.changed",
  "title.deleted",
  "episode.created",
  "episode.changed",
  "episode.deleted",
  "subtitle.created",
  "subtitle.deleted",
  "playlist.created",
  "playlist.changed",
  "playlist.deleted",
  "position.changed",
  "burn.running",
  "burn.finished",
  "burn.failed",
  "share.created",
  "share.deleted",
];

/// One event, and the account whose `mine` realm or position it belongs to, where it is one's.
#[derive(Clone, Debug)]
pub struct Record {
  pub event: StreamEvent,
  pub account: Option<String>,
}

/// The kept events and the live readers.
pub struct Events {
  kept: Mutex<(i64, VecDeque<Record>)>,
  first: i64,
  live: broadcast::Sender<Record>,
  stop: watch::Sender<bool>,
}

impl Default for Events {
  fn default() -> Self {
    Self::new()
  }
}

impl Events {
  pub fn new() -> Self {
    let first = now_ms() * 1000;
    let (live, _) = broadcast::channel(KEPT);
    let (stop, _) = watch::channel(false);
    Self {
      kept: Mutex::new((first, VecDeque::new())),
      first,
      live,
      stop,
    }
  }

  /// The number the first event of this run would take, less one: a resume from before it is gone.
  pub fn first(&self) -> i64 {
    self.first
  }

  /// One event published.
  pub fn publish(&self, kind: &str, data: impl Serialize, account: Option<String>) {
    let Ok(mut kept) = self.kept.lock() else {
      return;
    };
    kept.0 += 1;
    let record = Record {
      event: StreamEvent {
        id: kept.0,
        kind: kind.to_owned(),
        at: now_ms(),
        data: serde_json::to_value(data).unwrap_or_default(),
      },
      account,
    };
    kept.1.push_back(record.clone());
    while kept.1.len() > KEPT {
      kept.1.pop_front();
    }
    let _ = self.live.send(record);
  }

  /// The events kept after `after`, and whether `after` stands before the window (and so is gone).
  pub fn after(&self, after: i64) -> (Vec<Record>, bool) {
    let Ok(kept) = self.kept.lock() else {
      return (Vec::new(), false);
    };
    let oldest = kept.1.front().map(|record| record.event.id).unwrap_or(kept.0 + 1);
    let gone = after < self.first || after + 1 < oldest;
    (kept.1.iter().filter(|record| record.event.id > after).cloned().collect(), gone)
  }

  /// The last number given.
  pub fn latest(&self) -> i64 {
    self.kept.lock().map(|kept| kept.0).unwrap_or(self.first)
  }

  /// A reader of the events published from now.
  pub fn subscribe(&self) -> broadcast::Receiver<Record> {
    self.live.subscribe()
  }

  /// Ends every open stream.
  pub fn stop(&self) {
    let _ = self.stop.send(true);
  }

  /// Resolves once the streams are ended.
  pub fn stopped(&self) -> impl Future<Output = ()> + Send + 'static {
    let mut stop = self.stop.subscribe();
    async move {
      let _ = stop.wait_for(|stopped| *stopped).await;
    }
  }
}
