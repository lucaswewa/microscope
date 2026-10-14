//! The `smart_scan` Thing (P35, ADR-0029): scans with the chosen workflow,
//! one scan at a time. Each scan has a folder in the data folder's `scans/`:
//! its images in `images/`, each with its sidecar (ADR-0025), its record in
//! `scan.json`, and its log in `scan.log`. However a scan ends, the stage
//! goes back to where it started, and folders without an image are removed.
//! A scan can be downloaded as a ZIP archive (P35b).

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Cursor, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use microscope_core::scan::Finished;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use teta_wot::MediaType;
use teta_wot::prelude::*;

use crate::data;
use crate::hardware::{Position, StageApi};
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
    /// Held while a scan runs, so only one does.
    running: tokio::sync::Mutex<()>,
    latest: Mutex<Option<Latest>>,
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
        };
        *self.latest.lock().expect("not poisoned") = Some(Latest {
            details,
            started: Instant::now(),
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        });
        let log = ScanLog(folder.join("scan.log"));
        log.write(&format!("Scanning with {workflow_name} from {}", at(start)));
        let outcome = self
            .scan(&ctx, &*workflow, start, (&name, &folder), &log)
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
        let scan = data::data_folder(&server).join(SCANS).join(&scan_name);
        let mut parts = Path::new(&scan_name).components();
        let plain = matches!(
            (parts.next(), parts.next()),
            (Some(Component::Normal(_)), None)
        );
        if !plain || !scan.is_dir() {
            return Err(ActionError::handled(format!(
                "there's no scan `{scan_name}`"
            )));
        }
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

    /// Visits every site the workflow's planner gives, until it finishes.
    async fn scan(
        &self,
        ctx: &ActionCtx,
        workflow: &dyn ScanWorkflow,
        start: Position,
        (name, folder): (&str, &Path),
        log: &ScanLog,
    ) -> Result<Finished, ActionError> {
        let Begun {
            settings,
            mut planner,
        } = workflow.begin(start).await?;
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
            let acquired = workflow.acquire(images.clone()).await?;
            let (imaged, focused) = (acquired.images > 0, acquired.focus.is_some());
            planner.visited(acquired.focus.unwrap_or(z), imaged, focused);
            self.update(|details| details.image_count += acquired.images);
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

/// A new scan folder in `scans`, with its `images/`: called `requested`, or
/// by the time without it, with `-2`, `-3`, … if that's taken. Its name and
/// its path.
fn new_folder(scans: &Path, requested: &str) -> Result<(String, PathBuf), ActionError> {
    let requested = requested.trim();
    let stem = match requested {
        "" => format!("scan-{}", Utc::now().format("%Y-%m-%dT%H-%M-%SZ")),
        name => name.to_owned(),
    };
    let allowed = |c: char| c.is_alphanumeric() || " -_.".contains(c);
    if !stem.chars().all(allowed) || stem.starts_with('.') {
        return Err(ActionError::handled(
            "a scan's name can have letters, digits, spaces, `-`, `_` and `.`, and can't start with `.`",
        ));
    }
    std::fs::create_dir_all(scans).map_err(ActionError::handled)?;
    let mut n = 1;
    loop {
        let name = match n {
            1 => stem.clone(),
            n => format!("{stem}-{n}"),
        };
        let folder = scans.join(&name);
        match std::fs::create_dir(&folder) {
            Ok(()) => {
                std::fs::create_dir(folder.join("images")).map_err(ActionError::handled)?;
                return Ok((name, folder));
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => n += 1,
            Err(error) => return Err(ActionError::handled(error)),
        }
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
