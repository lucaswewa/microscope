//! The `gallery` Thing (P31, ADR-0026): one index of everything saved in
//! the data folder, for the Gallery tab. Each card type has a provider that
//! finds its items on disk, describes them and deletes them: the captures'
//! is here, and scans and sequences add theirs. The index is built when the
//! gallery starts, and brought up to date from the folders whenever it's
//! read, so it follows every write, by any Thing or from outside the server.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use teta_wot::http::axum::Json;
use teta_wot::http::axum::http::{StatusCode, Uri, header};
use teta_wot::http::axum::response::{IntoResponse, Response};
use teta_wot::prelude::*;

use crate::capture::{self, CAPTURES, Sidecar, StageRecord, THUMBNAILS};
use crate::data;
use crate::ui::{ActionButton, DownloadButton, DownloadSource};

/// What a URL's path segment can hold unencoded: letters, digits and `-._~`.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// One item in the gallery, as its card shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GalleryEntry {
    /// What the card is called.
    pub name: String,
    /// Where the item is in the data folder, as `GET {prefix}/data/{path}`
    /// serves it.
    pub path: String,
    /// What kind of item it is: one of `card_types`.
    pub card_type: String,
    /// When it was made: RFC 3339, in UTC, to the millisecond.
    pub created: String,
    /// Its thumbnail's path in the data folder, if it has one.
    pub thumbnail: Option<String>,
    /// How the viewer shows it.
    pub viewer: Viewer,
    /// Lines that describe it.
    pub info: Vec<InfoLine>,
    /// The ways to download it.
    pub download_buttons: Vec<DownloadButton>,
    /// What can be done with it.
    pub card_actions: Vec<ActionButton>,
    /// What the card warns of.
    pub card_messages: Vec<CardMessage>,
    /// Where to send `DELETE` to delete it, relative to the API prefix.
    pub delete_endpoint: String,
}

/// How the viewer shows an item (ADR-0027).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Viewer {
    /// As one image.
    Image {
        /// The image's path in the data folder.
        path: String,
    },
    /// As a deep-zoom image, such as a stitched scan (P38).
    Dzi {
        /// The path of its `.dzi` file, with its tiles beside it in
        /// `<name>_files/`.
        path: String,
    },
    /// As images shown one at a time, such as a timelapse's (P41).
    Directory {
        /// The images' paths, in order.
        paths: Vec<String>,
        /// The one shown first, from 0.
        start: usize,
    },
}

/// A line that describes an item, such as "Size: 1640 × 1232 px".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InfoLine {
    /// What it's about.
    pub label: String,
    /// What it says.
    pub value: String,
}

/// A message on a card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CardMessage {
    /// How serious it is.
    pub level: MessageLevel,
    /// What it says.
    pub message: String,
}

/// How serious a card's message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageLevel {
    /// Something to know.
    Warning,
    /// Something wrong.
    Alert,
}

/// What changes when an item's files do: each one's size and modification
/// time, if it's there.
type Stamp = Vec<Option<(u64, SystemTime)>>;

/// Finds one card type's items in the data folder, describes them and
/// deletes them.
trait Provider: Send + Sync {
    /// Its card type.
    fn card_type(&self) -> &'static str;
    /// Its items' paths in the data folder `folder`, each with a stamp of
    /// its files.
    fn items(&self, folder: &Path) -> Vec<(String, Stamp)>;
    /// The entry for the item at `path`, in the gallery called `gallery`.
    fn entry(&self, folder: &Path, path: &str, gallery: &str) -> GalleryEntry;
    /// Deletes the item at `path`, with all its files.
    fn delete(&self, folder: &Path, path: &str) -> std::io::Result<()>;
}

/// The providers, one for each card type.
const PROVIDERS: &[&dyn Provider] = &[&Captures];

/// The gallery (see the module's description).
#[derive(Thing)]
pub struct Gallery {
    /// The data folder and the gallery's name, once it has started.
    started: OnceLock<(PathBuf, String)>,
    /// The entries by path, each with the stamp it was described at.
    index: Mutex<BTreeMap<String, (Stamp, GalleryEntry)>>,
}

#[thing_impl]
impl Gallery {
    /// The kinds of item it shows.
    #[property]
    async fn card_types(&self) -> Vec<String> {
        PROVIDERS.iter().map(|p| p.card_type().to_owned()).collect()
    }

    /// Everything in the gallery, newest first.
    #[property(blocking)]
    fn list_data(&self) -> Vec<GalleryEntry> {
        self.refresh()
    }

    /// Actions on many items at once, for the gallery's sidebar. Captures
    /// have none.
    #[property]
    async fn bulk_actions(&self) -> Vec<ActionButton> {
        Vec::new()
    }

    /// Deletes every item of the given card types.
    #[action(blocking)]
    fn delete_all_data(&self, card_types: Vec<String>) -> Result<(), ActionError> {
        let known = |name: &String| PROVIDERS.iter().any(|p| p.card_type() == name);
        if let Some(unknown) = card_types.iter().find(|name| !known(name)) {
            return Err(ActionError::handled(format!(
                "there's no card type `{unknown}`"
            )));
        }
        let (folder, _) = self.started.get().ok_or_else(not_started)?;
        for provider in PROVIDERS {
            if card_types.iter().any(|name| name == provider.card_type()) {
                for (path, _) in provider.items(folder) {
                    provider
                        .delete(folder, &path)
                        .map_err(ActionError::handled)?;
                }
            }
        }
        Ok(())
    }

    /// Deletes the capture named by the path's last part, with its sidecar
    /// and its thumbnail.
    #[endpoint(delete, "captures/{id}")]
    async fn delete_capture(&self, uri: Uri) -> Response {
        let (folder, path) = match self.capture(&uri, 0) {
            Ok(found) => found,
            Err((status, message)) => return detail(status, message),
        };
        match Captures.delete(folder, &path) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => detail(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        }
    }

    /// The capture named by the path's second-last part, with its sidecar,
    /// as a ZIP archive.
    #[endpoint(get, "captures/{id}/zip")]
    async fn capture_zip(&self, uri: Uri) -> Response {
        let (folder, path) = match self.capture(&uri, 1) {
            Ok(found) => found,
            Err((status, message)) => return detail(status, message),
        };
        let image = folder.join(&path);
        let files: Vec<(String, PathBuf)> = [image.clone(), image.with_extension("json")]
            .into_iter()
            .filter(|file| file.is_file())
            .filter_map(|file| Some((file.file_name()?.to_str()?.to_owned(), file)))
            .collect();
        match data::zip(&files, Cursor::new(Vec::new())) {
            Ok(archive) => {
                let stem = image.file_stem().unwrap_or_default().to_string_lossy();
                let disposition = format!("attachment; filename=\"{stem}.zip\"");
                let headers = [
                    (header::CONTENT_TYPE, "application/zip".to_owned()),
                    (header::CONTENT_DISPOSITION, disposition),
                ];
                (headers, archive.into_inner()).into_response()
            }
            Err(error) => detail(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        }
    }

    /// Notes the data folder, and builds the index in the background.
    #[on_start]
    async fn start(&self, ctx: ThingCtx) {
        let started = (data::data_folder(ctx.server()), ctx.name().to_owned());
        let _ = self.started.set(started);
        if let Some(gallery) = ctx.server().thing_ref::<Gallery>(ctx.name()) {
            let gallery = Arc::clone(ThingRef::thing(&gallery));
            tokio::task::spawn_blocking(move || gallery.refresh());
        }
    }
}

impl Gallery {
    /// Brings the index up to date with the data folder, describing only
    /// the items whose files changed: every entry, newest first.
    fn refresh(&self) -> Vec<GalleryEntry> {
        let Some((folder, name)) = self.started.get() else {
            return Vec::new();
        };
        let mut index = self.index.lock().expect("not poisoned");
        let mut fresh = BTreeMap::new();
        for provider in PROVIDERS {
            for (path, stamp) in provider.items(folder) {
                let entry = match index.remove(&path) {
                    Some((old, entry)) if old == stamp => entry,
                    _ => provider.entry(folder, &path, name),
                };
                fresh.insert(path, (stamp, entry));
            }
        }
        *index = fresh;
        let mut entries: Vec<_> = index.values().map(|(_, entry)| entry.clone()).collect();
        entries.sort_by(|a, b| (&b.created, &b.path).cmp(&(&a.created, &a.path)));
        entries
    }

    /// The data folder, and the path in it of the capture named by the
    /// URL's path segment `back` from the end; or the status and message
    /// saying there's no such capture.
    fn capture(&self, uri: &Uri, back: usize) -> Result<(&Path, String), (StatusCode, String)> {
        let unstarted = || {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "the gallery hasn't started".to_owned(),
            )
        };
        let (folder, _) = self.started.get().ok_or_else(unstarted)?;
        let segment = uri.path().rsplit('/').nth(back).unwrap_or_default();
        let name = percent_decode_str(segment).decode_utf8_lossy();
        let mut parts = Path::new(name.as_ref()).components();
        let plain = matches!(
            (parts.next(), parts.next()),
            (Some(Component::Normal(_)), None)
        );
        let path = format!("{CAPTURES}/{name}");
        if plain && is_jpeg(&name) && folder.join(&path).is_file() {
            Ok((folder, path))
        } else {
            Err((
                StatusCode::NOT_FOUND,
                format!("There's no capture called `{name}`."),
            ))
        }
    }
}

/// The captures in `captures/`: each a JPEG with a sidecar (ADR-0025) and a
/// thumbnail.
struct Captures;

impl Provider for Captures {
    fn card_type(&self) -> &'static str {
        "Capture"
    }

    fn items(&self, folder: &Path) -> Vec<(String, Stamp)> {
        let Ok(files) = fs::read_dir(folder.join(CAPTURES)) else {
            return Vec::new();
        };
        let jpeg = |file: &fs::DirEntry| {
            let name = file.file_name().into_string().ok()?;
            let image = file.path();
            let is_file = file.file_type().ok()?.is_file();
            let stamp = stamp(&[&image, &image.with_extension("json")]);
            (is_file && is_jpeg(&name)).then(|| (format!("{CAPTURES}/{name}"), stamp))
        };
        files.flatten().filter_map(|file| jpeg(&file)).collect()
    }

    fn entry(&self, folder: &Path, path: &str, gallery: &str) -> GalleryEntry {
        let image = folder.join(path);
        let name = image.file_name().unwrap_or_default().to_string_lossy();
        let stem = image.file_stem().unwrap_or_default().to_string_lossy();
        let sidecar = fs::read(image.with_extension("json")).ok();
        let sidecar: Option<Sidecar> =
            sidecar.and_then(|bytes| serde_json::from_slice(&bytes).ok());
        // Captures from before P31 have no thumbnail yet.
        let thumbnail = capture::thumbnail_path(folder, &name);
        let thumbnail = thumbnail.is_file()
            || image::open(&image).is_ok_and(|picture| {
                capture::save_thumbnail(&picture.to_rgb8(), &thumbnail).is_ok()
            });
        let encoded = utf8_percent_encode(&name, SEGMENT);
        GalleryEntry {
            name: stem.clone().into_owned(),
            path: path.to_owned(),
            card_type: self.card_type().to_owned(),
            created: created(sidecar.as_ref(), &image),
            thumbnail: thumbnail.then(|| format!("{CAPTURES}/{THUMBNAILS}/{name}")),
            viewer: Viewer::Image {
                path: path.to_owned(),
            },
            info: sidecar.as_ref().map(info).unwrap_or_default(),
            download_buttons: vec![
                download(
                    "Image",
                    DownloadSource::Data {
                        path: path.to_owned(),
                    },
                    &name,
                ),
                DownloadButton {
                    secondary: true,
                    ..download(
                        "Image and metadata",
                        DownloadSource::Endpoint {
                            thing: gallery.to_owned(),
                            path: format!("{CAPTURES}/{encoded}/zip"),
                        },
                        &format!("{stem}.zip"),
                    )
                },
            ],
            card_actions: Vec::new(),
            card_messages: match sidecar {
                Some(_) => Vec::new(),
                None => vec![CardMessage {
                    level: MessageLevel::Warning,
                    message: "Its metadata is missing, so how it was taken isn't known.".to_owned(),
                }],
            },
            delete_endpoint: format!("{gallery}/{CAPTURES}/{encoded}"),
        }
    }

    fn delete(&self, folder: &Path, path: &str) -> std::io::Result<()> {
        let image = folder.join(path);
        fs::remove_file(&image)?;
        let name = image.file_name().unwrap_or_default().to_string_lossy();
        for file in [
            image.with_extension("json"),
            capture::thumbnail_path(folder, &name),
        ] {
            match fs::remove_file(file) {
                Err(error) if error.kind() != ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
    }
}

/// Each of `files`' size and modification time, if it's there.
fn stamp(files: &[&Path]) -> Stamp {
    let stamp = |file: &&Path| {
        let metadata = fs::metadata(file).ok()?;
        Some((metadata.len(), metadata.modified().ok()?))
    };
    files.iter().map(stamp).collect()
}

fn is_jpeg(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".jpeg") || name.ends_with(".jpg")
}

/// When a capture was taken, by its sidecar, else when its image was last
/// written.
fn created(sidecar: Option<&Sidecar>, image: &Path) -> String {
    let acquired = sidecar.and_then(|s| DateTime::parse_from_rfc3339(&s.acquired).ok());
    let time = acquired.map(|time| time.with_timezone(&Utc)).or_else(|| {
        let modified = fs::metadata(image).and_then(|m| m.modified()).ok()?;
        Some(DateTime::<Utc>::from(modified))
    });
    time.unwrap_or_default()
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// A capture's size, where it was taken, and through which objective.
fn info(sidecar: &Sidecar) -> Vec<InfoLine> {
    let line = |label: &str, value: String| InfoLine {
        label: label.to_owned(),
        value,
    };
    let mut lines = vec![line(
        "Size",
        format!("{} × {} px", sidecar.width, sidecar.height),
    )];
    if let Some(stage) = &sidecar.stage {
        lines.push(line("Position", position(stage)));
    }
    if let Some(objective) = sidecar.camera.get("objective").and_then(|o| o.as_f64()) {
        lines.push(line("Objective", format!("{objective}×")));
    }
    lines
}

/// Where the stage was: in µm if every axis's scale is known, else in steps.
fn position(stage: &StageRecord) -> String {
    match (stage.um.x, stage.um.y, stage.um.z) {
        (Some(x), Some(y), Some(z)) => format!("({x:.1}, {y:.1}, {z:.1}) µm"),
        _ => {
            let steps = stage.steps;
            format!("({}, {}, {}) steps", steps.x, steps.y, steps.z)
        }
    }
}

fn download(label: &str, source: DownloadSource, filename: &str) -> DownloadButton {
    DownloadButton {
        label: label.to_owned(),
        source,
        filename: filename.to_owned(),
        secondary: false,
        broken: false,
    }
}

fn not_started() -> ActionError {
    ActionError::handled("the gallery hasn't started")
}

/// An error answer, `{"detail": …}`, as `teta-wot`'s are.
fn detail(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "detail": message }))).into_response()
}
