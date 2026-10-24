//! The `smart_scan` Thing (P35, ADR-0029): scans with the chosen workflow,
//! one scan at a time. Each scan has a folder in the data folder's `scans/`:
//! its images in `images/`, each with its sidecar (ADR-0025), its record in
//! `scan.json`, and its log in `scan.log`. However a scan ends, the stage
//! goes back to where it started, and folders without an image are removed.
//! A scan can be downloaded as a ZIP archive (P35b). With
//! `autofocus_enabled` off (P51), the workflow doesn't autofocus: it takes
//! one image at each site where the stage is.
//!
//! Stitching (P39, [`crate::stitch`]): while a scan runs, its live preview
//! is drawn again after each site that saves an image, with where each image
//! went (P53). Once it ends, it's
//! stitched in the background if `stitch_automatically` is set, and its
//! images refined by correlation if `refine_with_correlation` is (P13).
//! `stitch_scan` and `stitch_all_scans` stitch scans on request. Like the
//! downloads, they don't take the global lock.

use std::collections::{BTreeMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use microscope_core::scan::Finished;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use teta_wot::MediaType;
use teta_wot::http::axum::Json;
use teta_wot::http::axum::http::{StatusCode, header};
use teta_wot::http::axum::response::{IntoResponse, Response};
use teta_wot::prelude::*;

use crate::data;
use crate::hardware::{Position, StageApi};
use crate::stitch::{self, Preview, STITCHED, Stitched, TileLayout};
use crate::workflow::{Begun, ScanWorkflow};

/// The folder, in the data folder, that scans are saved in.
pub const SCANS: &str = "scans";
/// The record's layout version.
const FORMAT: u32 = 1;

/// ZIP archives (`application/zip`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Zip;

impl MediaType for Zip {
    const MEDIA_TYPE: &'static str = "application/zip";
    const TITLE: &'static str = "ZIPBlob";
}

/// Where a scan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScanPhase {
    /// Getting ready, such as by autofocusing.
    Setup,
    /// Visiting its sites.
    Scanning,
    /// Ended, however it did.
    Complete,
}

/// How the latest scan is going, or went.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LiveScanDetails {
    /// Its name, which is its folder's.
    pub name: String,
    /// Where it is.
    pub scan_phase: ScanPhase,
    /// How many images it has saved.
    pub image_count: u32,
    /// How long it has run, in seconds.
    pub duration_seconds: f64,
    /// Its workflow, by the Thing's name.
    pub workflow: String,
    /// The workflow's settings for it, once it has begun.
    pub settings: Option<serde_json::Value>,
    /// How it ended: `success`, `cancelled`, or the error.
    pub result: Option<String>,
    /// When its live preview was last drawn: RFC 3339, in UTC.
    pub preview_stitch_time: Option<String>,
}

/// The latest scan's details, and when it started.
struct Latest {
    details: LiveScanDetails,
    started: Instant,
    /// When it started: RFC 3339, in UTC.
    started_at: String,
}

/// Scans with the chosen workflow: see the module's description.
#[derive(Thing)]
pub struct SmartScan {
    /// Every workflow, by its Thing's name.
    #[slot]
    workflows: SlotMap<dyn ScanWorkflow>,
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
    /// The workflow to scan with, by its Thing's name.
    #[setting(default = "histo_scan_workflow".to_owned())]
    workflow_name: Prop<String>,
    /// Whether each scan is stitched once it ends.
    #[setting(default = true)]
    stitch_automatically: Prop<bool>,
    /// Whether a scan's stitched mosaic has its images where their overlaps
    /// put them, by correlation, rather than where the stage put them
    /// (ADR-0046). The live preview keeps the stage's positions.
    #[setting(default = true)]
    refine_with_correlation: Prop<bool>,
    /// Whether a scan autofocuses: at its start, and at each site.
    /// Off, it takes one image at each site, where the stage is.
    #[setting(default = true)]
    autofocus_enabled: Prop<bool>,
    /// Held while a scan runs, so only one does.
    running: tokio::sync::Mutex<()>,
    latest: Mutex<Option<Latest>>,
    /// The latest scan's live preview, and its JPEG with where each image
    /// went.
    preview: Arc<Mutex<Preview>>,
    drawn_preview: Mutex<Option<(Vec<u8>, TileLayout)>>,
    /// The scans being stitched, by name.
    stitching: Arc<Mutex<HashSet<String>>>,
}

#[thing_impl]
impl SmartScan {
    /// The workflows' names.
    #[property]
    async fn all_workflow_names(&self) -> Vec<String> {
        self.workflows.keys().cloned().collect()
    }

    /// What the app calls each workflow, by its name.
    #[property]
    async fn workflow_display_names(&self) -> BTreeMap<String, String> {
        (self.workflows.iter())
            .map(|(name, workflow)| (name.clone(), workflow.display_name()))
            .collect()
    }

    /// What each workflow does, by its name, for the app to show.
    #[property]
    async fn workflow_descriptions(&self) -> BTreeMap<String, String> {
        (self.workflows.iter())
            .map(|(name, workflow)| (name.clone(), workflow.description()))
            .collect()
    }

    /// The latest scan's details, while it runs and after it ends.
    #[property]
    async fn latest_scan_live_details(&self) -> Option<LiveScanDetails> {
        let latest = self.latest.lock().expect("not poisoned");
        latest.as_ref().map(|latest| {
            let mut details = latest.details.clone();
            if details.scan_phase != ScanPhase::Complete {
                details.duration_seconds = latest.started.elapsed().as_secs_f64();
            }
            details
        })
    }

    /// The latest scan's live preview: its mosaic so far, as a JPEG.
    #[endpoint(get, "latest_preview_stitch.jpg")]
    async fn latest_preview_stitch(&self) -> Response {
        match self.drawn_preview.lock().expect("not poisoned").clone() {
            Some((jpeg, _)) => {
                let headers = [
                    (header::CONTENT_TYPE, "image/jpeg"),
                    (header::CACHE_CONTROL, "no-store"),
                ];
                (headers, jpeg).into_response()
            }
            None => {
                let detail = json!({"detail": "there's no preview yet"});
                (StatusCode::NOT_FOUND, Json(detail)).into_response()
            }
        }
    }

    /// Where each image is in the latest scan's live preview, in the pixels
    /// of the mosaic it was shrunk from (ADR-0036).
    #[endpoint(get, "latest_preview_tiles.json")]
    async fn latest_preview_tiles(&self) -> Response {
        match self.drawn_preview.lock().expect("not poisoned").clone() {
            Some((_, tiles)) => {
                ([(header::CACHE_CONTROL, "no-store")], Json(tiles)).into_response()
            }
            None => {
                let detail = json!({"detail": "there's no preview yet"});
                (StatusCode::NOT_FOUND, Json(detail)).into_response()
            }
        }
    }

    /// Scans with the chosen workflow into a new folder in `scans/`, called
    /// `scan_name`, or by the time without one, with `-2`, `-3`, … if that's
    /// taken. However it ends, the stage goes back to the start. Returns the
    /// folder's name.
    #[action]
    async fn sample_scan(
        &self,
        ctx: ActionCtx,
        #[param(default)] scan_name: String,
    ) -> Result<String, ActionError> {
        let Ok(_running) = self.running.try_lock() else {
            return Err(ActionError::handled("a scan is already running"));
        };
        let workflow_name = self.workflow_name.get();
        let workflow = (self.workflows.iter())
            .find(|(name, _)| **name == workflow_name)
            .map(|(_, workflow)| Arc::clone(workflow))
            .ok_or_else(|| {
                ActionError::handled(format!("there's no workflow `{workflow_name}`"))
            })?;
        if !workflow.ready() {
            let name = workflow.display_name();
            return Err(ActionError::handled(format!("{name} isn't ready to scan")));
        }
        let scans = data::data_folder(ctx.server()).join(SCANS);
        let (name, folder) = new_folder(&scans, &scan_name)?;
        let start = self.stage.position().await?;
        let details = LiveScanDetails {
            name: name.clone(),
            scan_phase: ScanPhase::Setup,
            image_count: 0,
            duration_seconds: 0.0,
            workflow: workflow_name.clone(),
            settings: None,
            result: None,
            preview_stitch_time: None,
        };
        *self.preview.lock().expect("not poisoned") = Preview::default();
        *self.drawn_preview.lock().expect("not poisoned") = None;
        *self.latest.lock().expect("not poisoned") = Some(Latest {
            details,
            started: Instant::now(),
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        });
        let log = ScanLog(folder.join("scan.log"));
        log.write(&format!("Scanning with {workflow_name} from {}", at(start)));
        let focus = self.autofocus_enabled.get();
        if !focus {
            log.write("Autofocus is off: one image at each site, where the stage is");
        }
        let outcome = self
            .scan(&ctx, (&*workflow, focus), start, (&name, &folder), &log)
            .await;
        log.write("Going back to the start");
        let back = self.stage.move_absolute(start).await;
        let result = match &outcome {
            Ok(_) => "success".to_owned(),
            Err(error) if error.is_cancelled() => "cancelled".to_owned(),
            Err(error) => format!("error: {error}"),
        };
        if let Ok(finished) = &outcome {
            log.write(reason(*finished));
        }
        log.write(&format!("Ended: {result}"));
        self.update(|details| {
            details.scan_phase = ScanPhase::Complete;
            details.result = Some(result);
        });
        self.record(&folder, start)?;
        purge(&scans, None)?;
        let imaged = self
            .latest_details()
            .is_some_and(|(d, _)| d.image_count > 0);
        if imaged && self.stitch_automatically.get() {
            self.stitch_later(&name, folder);
        }
        outcome?;
        back?;
        Ok(name)
    }

    /// Removes every scan folder without an image, but the one being
    /// scanned. Returns how many it removed.
    #[action(blocking)]
    fn purge_empty_scans(&self, server: Server) -> Result<u32, ActionError> {
        let scans = data::data_folder(&server).join(SCANS);
        purge(&scans, self.scanning().as_deref())
    }

    /// The scan `scan_name` as a ZIP archive: its images with their
    /// sidecars, its record and its log.
    #[action(blocking, global_lock = false)]
    fn download_zip(&self, server: Server, scan_name: String) -> Result<Blob<Zip>, ActionError> {
        let scan = scan_folder(&server, &scan_name)?;
        let mut files: Vec<(String, PathBuf)> = ["scan.json", "scan.log"]
            .into_iter()
            .map(|name| (name.to_owned(), scan.join(name)))
            .filter(|(_, path)| path.is_file())
            .collect();
        let images = std::fs::read_dir(scan.join("images")).map_err(ActionError::handled)?;
        let mut images: Vec<String> = (images.flatten())
            .filter_map(|file| file.file_name().into_string().ok())
            .collect();
        images.sort();
        files.extend(
            images
                .into_iter()
                .map(|image| (format!("images/{image}"), scan.join("images").join(image))),
        );
        let archive = data::zip(&files, Cursor::new(Vec::new())).map_err(ActionError::handled)?;
        Ok(Blob::from_bytes(archive.into_inner()))
    }

    /// Stitches the scan `scan_name` into its `stitched/` folder, replacing
    /// any earlier stitch.
    #[action(blocking, global_lock = false)]
    fn stitch_scan(&self, server: Server, scan_name: String) -> Result<Stitched, ActionError> {
        let scan = scan_folder(&server, &scan_name)?;
        if self.scanning().as_deref() == Some(scan_name.as_str()) {
            return Err(ActionError::handled(format!(
                "`{scan_name}` is still scanning"
            )));
        }
        let _stitching = Stitching::start(&self.stitching, &scan_name).ok_or_else(|| {
            ActionError::handled(format!("`{scan_name}` is already being stitched"))
        })?;
        stitch::stitch(&scan, self.refine_with_correlation.get())
    }

    /// Stitches every scan that isn't stitched, but the one being scanned.
    /// Returns the names of those it stitched; any that fail are logged.
    #[action(blocking, global_lock = false)]
    fn stitch_all_scans(&self, server: Server) -> Result<Vec<String>, ActionError> {
        let scans = data::data_folder(&server).join(SCANS);
        let mut names: Vec<String> = (std::fs::read_dir(&scans).into_iter().flatten().flatten())
            .filter(|entry| entry.path().is_dir() && !entry.path().join(STITCHED).exists())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        let mut stitched = Vec::new();
        for name in names {
            let running = self.scanning().as_deref() == Some(name.as_str());
            let Some(_stitching) = Stitching::start(&self.stitching, &name).filter(|_| !running)
            else {
                continue;
            };
            match stitch::stitch(&scans.join(&name), self.refine_with_correlation.get()) {
                Ok(_) => stitched.push(name),
                Err(error) => tracing::warn!("couldn't stitch `{name}`: {error}"),
            }
        }
        Ok(stitched)
    }

    /// The scan `scan_name`'s stitched mosaic as a ZIP archive: its
    /// deep-zoom image, overview and thumbnail.
    #[action(blocking, global_lock = false)]
    fn download_stitched_zip(
        &self,
        server: Server,
        scan_name: String,
    ) -> Result<Blob<Zip>, ActionError> {
        let stitched = scan_folder(&server, &scan_name)?.join(STITCHED);
        if !stitched.is_dir() {
            return Err(ActionError::handled(format!(
                "`{scan_name}` isn't stitched"
            )));
        }
        let mut files = Vec::new();
        files_in(&stitched, "", &mut files).map_err(ActionError::handled)?;
        files.sort();
        let archive = data::zip(&files, Cursor::new(Vec::new())).map_err(ActionError::handled)?;
        Ok(Blob::from_bytes(archive.into_inner()))
    }
}

impl SmartScan {
    /// The name of the scan being run, if one is.
    pub fn scanning(&self) -> Option<String> {
        let latest = self.latest.lock().expect("not poisoned");
        let running = latest
            .as_ref()
            .filter(|l| l.details.scan_phase != ScanPhase::Complete);
        running.map(|latest| latest.details.name.clone())
    }

    /// Stitches the scan `name`, in `folder`, in the background, and says
    /// how that went in its log.
    fn stitch_later(&self, name: &str, folder: PathBuf) {
        let Some(stitching) = Stitching::start(&self.stitching, name) else {
            return;
        };
        let refine = self.refine_with_correlation.get();
        tokio::task::spawn_blocking(move || {
            let _stitching = stitching;
            let log = ScanLog(folder.join("scan.log"));
            match stitch::stitch(&folder, refine) {
                Ok(stitched) => log.write(&stitched.summary()),
                Err(error) => log.write(&format!("Stitching failed: {error}")),
            }
        });
    }

    /// Draws the live preview again from the scan's images in `folder`. If
    /// it can't, that's logged, and the scan carries on.
    async fn draw_preview(&self, folder: &Path) {
        let (preview, folder) = (Arc::clone(&self.preview), folder.to_owned());
        let drawn = tokio::task::spawn_blocking(move || {
            preview.lock().expect("not poisoned").update(&folder)
        });
        match drawn
            .await
            .map_err(ActionError::handled)
            .and_then(|drawn| drawn)
        {
            Ok(drawn) => {
                *self.drawn_preview.lock().expect("not poisoned") = Some(drawn);
                let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
                self.update(|details| details.preview_stitch_time = Some(now));
            }
            Err(error) => tracing::warn!("the live preview couldn't be drawn: {error}"),
        }
    }

    /// Visits every site the workflow's planner gives, until it finishes.
    async fn scan(
        &self,
        ctx: &ActionCtx,
        (workflow, focus): (&dyn ScanWorkflow, bool),
        start: Position,
        (name, folder): (&str, &Path),
        log: &ScanLog,
    ) -> Result<Finished, ActionError> {
        let Begun {
            mut settings,
            mut planner,
        } = workflow.begin(start, focus).await?;
        if let Some(settings) = settings.as_object_mut() {
            settings.insert("autofocus_enabled".to_owned(), focus.into());
        }
        self.update(|details| {
            details.settings = Some(settings);
            details.scan_phase = ScanPhase::Scanning;
        });
        self.record(folder, start)?;
        // In the data folder, as stacks take it.
        let images = format!("{SCANS}/{name}/images");
        while let Some((site, z)) = planner.next() {
            ctx.check_cancelled()?;
            let z = match z {
                Some(z) => z,
                None => self.stage.position().await?.z,
            };
            let target = Position::new(site[0], site[1], z);
            log.write(&format!("Moving to {}", at(target)));
            self.stage.move_absolute(target).await?;
            let acquired = workflow.acquire(images.clone(), focus).await?;
            let (imaged, focused) = (acquired.images > 0, acquired.focus.is_some());
            planner.visited(acquired.focus.unwrap_or(z), imaged, focused);
            self.update(|details| details.image_count += acquired.images);
            if imaged {
                self.draw_preview(folder).await;
            }
        }
        Ok(planner.finished().unwrap_or(Finished::AllVisited))
    }

    fn update(&self, change: impl FnOnce(&mut LiveScanDetails)) {
        if let Some(latest) = self.latest.lock().expect("not poisoned").as_mut() {
            change(&mut latest.details);
            latest.details.duration_seconds = latest.started.elapsed().as_secs_f64();
        }
    }

    /// Writes the scan's record, `scan.json`, from its details.
    fn record(&self, folder: &Path, start: Position) -> Result<(), ActionError> {
        let Some((details, started)) = self.latest_details() else {
            return Ok(());
        };
        let record = json!({
            "format": FORMAT,
            "name": details.name,
            "workflow": details.workflow,
            "settings": details.settings,
            "start": start,
            "started": started,
            "duration_seconds": details.duration_seconds,
            "image_count": details.image_count,
            "result": details.result,
        });
        let text = serde_json::to_vec_pretty(&record).map_err(ActionError::handled)?;
        std::fs::write(folder.join("scan.json"), text).map_err(ActionError::handled)
    }

    /// The latest scan's details, and when it started.
    fn latest_details(&self) -> Option<(LiveScanDetails, String)> {
        let latest = self.latest.lock().expect("not poisoned");
        let latest = latest.as_ref()?;
        Some((latest.details.clone(), latest.started_at.clone()))
    }
}

/// A new scan folder in `scans`, with its `images/`.
fn new_folder(scans: &Path, requested: &str) -> Result<(String, PathBuf), ActionError> {
    let (name, folder) = data::new_folder(scans, requested, "scan")?;
    std::fs::create_dir(folder.join("images")).map_err(ActionError::handled)?;
    Ok((name, folder))
}

/// The scan `name`'s folder, if it has one: a plain name, in `scans/`.
fn scan_folder(server: &Server, name: &str) -> Result<PathBuf, ActionError> {
    let scan = data::data_folder(server).join(SCANS).join(name);
    let mut parts = Path::new(name).components();
    let plain = matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    );
    if !plain || !scan.is_dir() {
        return Err(ActionError::handled(format!("there's no scan `{name}`")));
    }
    Ok(scan)
}

/// Every file in `folder` and below, each with its path from there after
/// `prefix`.
fn files_in(
    folder: &Path,
    prefix: &str,
    files: &mut Vec<(String, PathBuf)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(folder)?.flatten() {
        let (path, name) = (entry.path(), entry.file_name());
        let inside = format!("{prefix}{}", name.to_string_lossy());
        if path.is_dir() {
            files_in(&path, &format!("{inside}/"), files)?;
        } else {
            files.push((inside, path));
        }
    }
    Ok(())
}

/// A scan being stitched: its name is in the set until this is dropped.
struct Stitching {
    set: Arc<Mutex<HashSet<String>>>,
    name: String,
}

impl Stitching {
    /// Marks `name` as being stitched, unless it already is.
    fn start(set: &Arc<Mutex<HashSet<String>>>, name: &str) -> Option<Stitching> {
        let added = set.lock().expect("not poisoned").insert(name.to_owned());
        added.then(|| Stitching {
            set: Arc::clone(set),
            name: name.to_owned(),
        })
    }
}

impl Drop for Stitching {
    fn drop(&mut self) {
        self.set.lock().expect("not poisoned").remove(&self.name);
    }
}

/// Removes each scan folder in `scans` without an image in its `images/`,
/// but `except`. Returns how many it removed.
fn purge(scans: &Path, except: Option<&str>) -> Result<u32, ActionError> {
    let Ok(folders) = std::fs::read_dir(scans) else {
        return Ok(0);
    };
    let has_image = |scan: &Path| {
        let images = std::fs::read_dir(scan.join("images"))
            .into_iter()
            .flatten()
            .flatten();
        images
            .into_iter()
            .any(|file| file.path().extension().is_some_and(|e| e == "jpeg"))
    };
    let mut removed = 0;
    for folder in folders.flatten() {
        let (name, path) = (folder.file_name(), folder.path());
        if except.is_some_and(|except| name == except) || !path.is_dir() || has_image(&path) {
            continue;
        }
        std::fs::remove_dir_all(&path).map_err(ActionError::handled)?;
        removed += 1;
    }
    Ok(removed)
}

/// A scan's log, `scan.log`: each line timed. Each also goes to the
/// action's log.
struct ScanLog(PathBuf);

impl ScanLog {
    fn write(&self, line: &str) {
        tracing::info!("{line}");
        let time = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let file = OpenOptions::new().create(true).append(true).open(&self.0);
        if let Ok(mut file) = file {
            let _ = writeln!(file, "{time} {line}");
        }
    }
}

fn at(position: Position) -> String {
    format!("x {}, y {}, z {}", position.x, position.y, position.z)
}

fn reason(finished: Finished) -> &'static str {
    match finished {
        Finished::AllVisited => "Every planned site was visited",
        Finished::ReachedRange => "It reached its maximum range",
        Finished::SurroundedByBackground => "The scanned area is surrounded by background",
    }
}
