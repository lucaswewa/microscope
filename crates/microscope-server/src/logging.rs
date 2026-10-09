//! Server-wide logging (ADR-0008).
//!
//! Every `tracing` event can go to four places:
//!
//! - **the console**, filtered by `RUST_LOG` (`info` by default, `debug`
//!   with `--debug`);
//! - **the log of the invocation** it happened in, if any, through
//!   `teta-wot`'s invocation-log layer;
//! - **the server log**: the last [`CAPACITY`] records, in memory, served at
//!   `{prefix}/log/`;
//! - **the log file**: one a day in the configured `log_folder`, served at
//!   `{prefix}/logfile/`.
//!
//! The server log and the log file capture INFO and more severe events, and
//! with `--debug` also DEBUG events from this application and `teta-wot`
//! ([`capture_filter`]).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use chrono::Utc;
use serde::Serialize;
use teta_wot::invocation::InvocationManager;
use teta_wot::logs::{INVOCATION_SPAN, LogRecord, python_level};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::{EnvFilter, LevelFilter, Targets};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, fmt};
use uuid::Uuid;

/// How many records the server log keeps.
pub const CAPACITY: usize = 1000;

/// Log files are named `microscope.YYYY-MM-DD.log`, with the date in UTC.
pub const FILE_PREFIX: &str = "microscope";

/// See [`FILE_PREFIX`].
pub const FILE_SUFFIX: &str = "log";

/// How many daily log files are kept; older ones are deleted.
pub const MAX_FILES: usize = 30;

/// The targets whose DEBUG events are captured with `--debug`.
const DEBUG_TARGETS: [&str; 9] = [
    "microscope_core",
    "microscope_sim",
    "microscope_things",
    "microscope_server",
    "teta_wot",
    "teta_wot_core",
    "teta_wot_http",
    "teta_wot_server",
    "teta_wot_td",
];

/// One record of the server log: the record `teta-wot` keeps in invocation
/// logs, with where it came from.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServerLogRecord {
    /// Counts records from 1, in the order they were logged.
    pub sequence: u64,
    /// Where the event came from: its `tracing` target, usually a module
    /// path (Python's logger name).
    pub name: String,
    /// The message, level, time, source and error, as in invocation logs.
    #[serde(flatten)]
    pub record: LogRecord,
    /// The invocation the event happened in, if any.
    pub invocation_id: Option<Uuid>,
    /// The Thing of that invocation, if it is known.
    pub thing: Option<String>,
    /// The action of that invocation, if it is known.
    pub action: Option<String>,
}

/// The server log, and where log files are written.
#[derive(Debug, Clone)]
pub struct Logs {
    shared: Arc<Shared>,
    folder: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct Shared {
    records: Mutex<VecDeque<ServerLogRecord>>,
    next_sequence: AtomicU64,
    invocations: OnceLock<Arc<InvocationManager>>,
}

impl Logs {
    /// An empty server log, with log files in `folder` if it is given.
    ///
    /// Events reach it only through its [`layer`](Self::layer), which
    /// [`init`] installs.
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            shared: Arc::default(),
            folder,
        }
    }

    /// Where log files are written, if anywhere.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Lets records name the Thing and action of their invocation. Called
    /// once the server is built; later calls are ignored.
    pub fn attach(&self, invocations: Arc<InvocationManager>) {
        let _ = self.shared.invocations.set(invocations);
    }

    /// The records at `min_levelno` or more severe, oldest first.
    pub fn records(&self, min_levelno: u8) -> Vec<ServerLogRecord> {
        self.shared
            .lock()
            .iter()
            .filter(|r| r.record.levelno >= min_levelno)
            .cloned()
            .collect()
    }

    /// The records as text, one line each, oldest first: for the fallback
    /// page.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for r in self.shared.lock().iter() {
            let created = r.record.created.format("%Y-%m-%dT%H:%M:%S%.6fZ");
            let _ = writeln!(
                text,
                "{created} [{}] <{}> {}",
                r.record.levelname, r.name, r.record.message
            );
        }
        text
    }

    /// The newest log file, if any has been written. Dates in the names
    /// sort in time order.
    pub fn current_file(&self) -> Option<PathBuf> {
        let prefix = format!("{FILE_PREFIX}.");
        let suffix = format!(".{FILE_SUFFIX}");
        std::fs::read_dir(self.folder.as_ref()?)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(&suffix))
            })
            .max()
    }

    /// The layer that copies events into this log.
    pub fn layer<S>(&self) -> impl Layer<S> + use<S>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        ServerLogLayer {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, VecDeque<ServerLogRecord>> {
        self.records.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Installs the process's `tracing` subscriber, with all four outputs, and
/// returns the server log.
///
/// If `log_folder` can't be created or written, logging carries on without
/// files, and says so. If a subscriber is already installed, nothing is
/// installed, and nothing reaches the returned log.
pub fn init(debug: bool, log_folder: &Path) -> Logs {
    let (appender, file_error) = match file_appender(log_folder) {
        Ok(appender) => (Some(appender), None),
        Err(error) => (None, Some(error)),
    };
    let logs = Logs::new(appender.is_some().then(|| log_folder.to_path_buf()));

    let console = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(if debug { "debug" } else { "info" }));
    let file = appender.map(|appender| {
        fmt::layer()
            .with_ansi(false)
            .with_writer(appender)
            .with_filter(capture_filter(debug))
    });
    let installed = tracing_subscriber::registry()
        .with(fmt::layer().with_filter(console))
        .with(teta_wot::logging::invocation_log_layer(debug))
        .with(logs.layer().with_filter(capture_filter(debug)))
        .with(file)
        .try_init();

    if let Err(error) = installed {
        eprintln!("Warning: logging wasn't set up: {error}");
    }
    if let Some(error) = file_error {
        tracing::warn!(
            "log files can't be written to {}: {error}",
            log_folder.display()
        );
    }
    logs
}

fn file_appender(folder: &Path) -> Result<RollingFileAppender, String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(FILE_PREFIX)
        .filename_suffix(FILE_SUFFIX)
        .max_log_files(MAX_FILES)
        .build(folder)
        .map_err(|e| e.to_string())
}

/// What the server log and the log file capture: INFO and more severe, and
/// with `debug` also DEBUG from this application and `teta-wot` (not from
/// libraries such as hyper, which would crowd out everything else).
pub fn capture_filter(debug: bool) -> Targets {
    let mut targets = Targets::new().with_default(LevelFilter::INFO);
    if debug {
        for target in DEBUG_TARGETS {
            targets = targets.with_target(target, LevelFilter::DEBUG);
        }
    }
    targets
}

/// A level's number from its Python name (`DEBUG`, `INFO`, `WARNING`,
/// `ERROR`, `CRITICAL`; also `TRACE` and `WARN`), in any case, or from a
/// number.
pub fn parse_level(level: &str) -> Option<u8> {
    match level.trim().to_ascii_uppercase().as_str() {
        "TRACE" => Some(5),
        "DEBUG" => Some(10),
        "INFO" => Some(20),
        "WARNING" | "WARN" => Some(30),
        "ERROR" => Some(40),
        "CRITICAL" => Some(50),
        number => number.parse().ok(),
    }
}

struct ServerLogLayer {
    shared: Arc<Shared>,
}

/// Stored in the extensions of `teta-wot`'s invocation spans.
#[derive(Debug, Clone, Copy)]
struct InvocationSpan(Uuid);

impl<S> Layer<S> for ServerLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != INVOCATION_SPAN {
            return;
        }
        let mut visitor = IdVisitor(None);
        attrs.record(&mut visitor);
        if let Some(invocation) = visitor.0
            && let Some(span) = ctx.span(id)
        {
            span.extensions_mut().insert(InvocationSpan(invocation));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let (invocation_id, thing, action) = self.invocation_of(event, &ctx);
        let record = ServerLogRecord {
            sequence: self.shared.next_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            name: event.metadata().target().to_owned(),
            record: record_for(event),
            invocation_id,
            thing,
            action,
        };
        let mut records = self.shared.lock();
        if records.len() == CAPACITY {
            records.pop_front();
        }
        records.push_back(record);
    }
}

impl ServerLogLayer {
    /// The invocation an event happened in, with its Thing and action.
    ///
    /// The innermost invocation span that the invocation manager knows wins,
    /// so events from in-process calls are credited to the action a client
    /// invoked. If the manager knows none (or isn't attached yet), the
    /// innermost invocation's ID is kept without names.
    ///
    /// This looks invocations up while an event is being recorded. That is
    /// safe because `InvocationManager` (at `teta-wot` v0.1.0) never logs while
    /// holding its lock. Check again when upgrading `teta-wot`.
    fn invocation_of<S>(
        &self,
        event: &Event<'_>,
        ctx: &Context<'_, S>,
    ) -> (Option<Uuid>, Option<String>, Option<String>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let Some(scope) = ctx.event_scope(event) else {
            return (None, None, None);
        };
        let manager = self.shared.invocations.get();
        let mut innermost = None;
        for span in scope {
            let Some(InvocationSpan(id)) = span.extensions().get::<InvocationSpan>().copied()
            else {
                continue;
            };
            innermost.get_or_insert(id);
            if let Some(invocation) = manager.and_then(|m| m.get(id)) {
                return (
                    Some(id),
                    Some(invocation.thing().to_owned()),
                    Some(invocation.action().to_owned()),
                );
            }
        }
        (innermost, None, None)
    }
}

struct IdVisitor(Option<Uuid>);

impl Visit for IdVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "invocation_id" {
            self.0 = value.parse().ok();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "invocation_id" {
            self.0 = format!("{value:?}").parse().ok();
        }
    }
}

/// Formats an event as `teta-wot` does in invocation logs: the message,
/// then the other fields as ` key=value`, with `exception_type` and
/// `traceback` kept apart.
#[derive(Default)]
struct RecordVisitor {
    message: String,
    fields: String,
    exception_type: Option<String>,
    traceback: Option<String>,
}

impl RecordVisitor {
    fn record(&mut self, field: &Field, value: String) {
        match field.name() {
            "message" => self.message = value,
            "exception_type" => self.exception_type = Some(value),
            "traceback" => self.traceback = Some(value),
            name if name.starts_with("log.") => {}
            name => {
                let _ = write!(self.fields, " {name}={value}");
            }
        }
    }
}

impl Visit for RecordVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }
}

fn record_for(event: &Event<'_>) -> LogRecord {
    let metadata = event.metadata();
    let mut visitor = RecordVisitor::default();
    event.record(&mut visitor);
    let (levelname, levelno) = python_level(metadata.level());
    let filename = metadata
        .file()
        .map(|f| f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned())
        .unwrap_or_default();
    LogRecord {
        message: visitor.message + &visitor.fields,
        levelname: levelname.to_owned(),
        levelno,
        lineno: metadata.line().unwrap_or(0),
        filename,
        created: Utc::now(),
        exception_type: visitor.exception_type,
        traceback: visitor.traceback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `f` with a subscriber that has only `logs`'s layer, filtered as
    /// the server filters it.
    fn capture(logs: &Logs, debug: bool, f: impl FnOnce()) {
        let subscriber =
            tracing_subscriber::registry().with(logs.layer().with_filter(capture_filter(debug)));
        tracing::subscriber::with_default(subscriber, f);
    }

    #[test]
    fn records_are_kept_oldest_first_up_to_the_capacity() {
        let logs = Logs::new(None);
        capture(&logs, false, || {
            for i in 0..CAPACITY + 5 {
                tracing::info!("event {i}");
            }
        });
        let records = logs.records(0);
        assert_eq!(records.len(), CAPACITY);
        assert_eq!(records[0].sequence, 6);
        assert_eq!(records[0].record.message, "event 5");
        let last = records.last().expect("records");
        assert_eq!(last.sequence, (CAPACITY + 5) as u64);
        assert_eq!(last.record.message, format!("event {}", CAPACITY + 4));
    }

    #[test]
    fn a_record_has_the_server_log_shape() {
        let logs = Logs::new(None);
        capture(&logs, false, || {
            tracing::warn!(target: "microscope_server::example", count = 3, "something happened");
        });
        let record = &logs.records(0)[0];
        assert_eq!(record.sequence, 1);
        assert_eq!(record.name, "microscope_server::example");
        assert_eq!(record.record.message, "something happened count=3");
        assert_eq!(record.record.levelname, "WARNING");
        assert_eq!(record.record.levelno, 30);
        assert_eq!(record.record.filename, "logging.rs");
        assert!(record.record.lineno > 0);
        assert_eq!(record.invocation_id, None);

        let json = serde_json::to_value(record).expect("serialisable");
        let mut keys: Vec<_> = json
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "action",
                "created",
                "exception_type",
                "filename",
                "invocation_id",
                "levelname",
                "levelno",
                "lineno",
                "message",
                "name",
                "sequence",
                "thing",
                "traceback",
            ]
        );
        assert!(json["created"].as_str().is_some_and(|t| t.ends_with('Z')));
    }

    #[test]
    fn errors_keep_their_type_and_traceback_apart() {
        let logs = Logs::new(None);
        capture(&logs, false, || {
            tracing::error!(
                exception_type = "IoError",
                traceback = "disk full\ncaused by: quota",
                "saving failed"
            );
        });
        let record = &logs.records(0)[0].record;
        assert_eq!(record.message, "saving failed");
        assert_eq!(record.exception_type.as_deref(), Some("IoError"));
        assert_eq!(
            record.traceback.as_deref(),
            Some("disk full\ncaused by: quota")
        );
    }

    #[test]
    fn records_can_be_filtered_by_level() {
        let logs = Logs::new(None);
        capture(&logs, false, || {
            tracing::info!("routine");
            tracing::warn!("odd");
            tracing::error!("broken");
        });
        let messages = |min| -> Vec<String> {
            logs.records(min)
                .into_iter()
                .map(|r| r.record.message)
                .collect()
        };
        assert_eq!(messages(0), ["routine", "odd", "broken"]);
        assert_eq!(messages(30), ["odd", "broken"]);
        assert_eq!(messages(50), Vec::<String>::new());
    }

    #[test]
    fn events_in_invocation_spans_carry_the_innermost_invocation_id() {
        let logs = Logs::new(None);
        let outer = Uuid::new_v4();
        let inner = Uuid::new_v4();
        capture(&logs, false, || {
            tracing::info_span!("invocation", invocation_id = %outer).in_scope(|| {
                tracing::info!("in the outer invocation");
                tracing::info_span!("invocation", invocation_id = %inner).in_scope(|| {
                    tracing::info!("in the inner invocation");
                });
            });
            tracing::info!("outside");
        });
        let records = logs.records(0);
        assert_eq!(records[0].invocation_id, Some(outer));
        assert_eq!(records[1].invocation_id, Some(inner));
        assert_eq!(records[2].invocation_id, None);
        // No invocation manager is attached, so no names are known.
        assert!(
            records
                .iter()
                .all(|r| r.thing.is_none() && r.action.is_none())
        );
    }

    #[test]
    fn debug_mode_captures_debug_events_from_the_application_only() {
        for debug in [false, true] {
            let logs = Logs::new(None);
            capture(&logs, debug, || {
                tracing::debug!(target: "microscope_server::cli", "ours");
                tracing::debug!(target: "teta_wot_core::runtime", "teta-wot's");
                tracing::debug!(target: "hyper::proto", "a library's");
                tracing::info!(target: "hyper::proto", "a library's info");
            });
            let messages: Vec<_> = logs
                .records(0)
                .into_iter()
                .map(|r| r.record.message)
                .collect();
            if debug {
                assert_eq!(messages, ["ours", "teta-wot's", "a library's info"]);
            } else {
                assert_eq!(messages, ["a library's info"]);
            }
        }
    }

    #[test]
    fn levels_parse_from_names_and_numbers() {
        assert_eq!(parse_level("debug"), Some(10));
        assert_eq!(parse_level("INFO"), Some(20));
        assert_eq!(parse_level("Warning"), Some(30));
        assert_eq!(parse_level("warn"), Some(30));
        assert_eq!(parse_level("ERROR"), Some(40));
        assert_eq!(parse_level("critical"), Some(50));
        assert_eq!(parse_level("25"), Some(25));
        assert_eq!(parse_level("loud"), None);
    }

    #[test]
    fn the_current_file_is_the_newest_log_file() {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let logs = Logs::new(Some(folder.path().to_path_buf()));
        assert_eq!(logs.current_file(), None);
        for name in [
            "microscope.2026-10-09.log",
            "microscope.2026-10-10.log",
            "microscope.2026-09-30.log",
            "other.2026-12-31.log",
            "microscope.2026-12-31.txt",
        ] {
            std::fs::write(folder.path().join(name), "").expect("writable");
        }
        assert_eq!(
            logs.current_file(),
            Some(folder.path().join("microscope.2026-10-10.log"))
        );
        assert_eq!(Logs::new(None).current_file(), None);
    }

    #[test]
    fn text_has_one_line_per_record() {
        let logs = Logs::new(None);
        capture(&logs, false, || {
            tracing::info!(target: "microscope_server::cli", "first");
            tracing::warn!(target: "microscope_server::cli", "second");
        });
        let text = logs.text();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with(" [INFO] <microscope_server::cli> first"));
        assert!(lines[1].ends_with(" [WARNING] <microscope_server::cli> second"));
    }
}
