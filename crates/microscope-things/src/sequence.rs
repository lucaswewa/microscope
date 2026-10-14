//! The `sequence` Thing (P41, ADR-0031): timelapses, as plans of time points
//! and the steps at each (`microscope_core::sequence`). Each sequence has a
//! folder in the data folder's `sequences/`: its images, `image_<k>.jpeg`
//! for time point k, each with its sidecar (ADR-0025), and its record,
//! `sequence.json`. Captures are kept in memory and saved by a queue beside
//! the schedule. The behaviour spec opens the P41 notes. How the latest
//! sequence is going is `latest_sequence_live_details`, which the Sequence
//! tab follows (P42).

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use microscope_core::sequence::{Due, SequencePlan, Step};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use teta_wot::prelude::*;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::autofocus::{Autofocus, AutofocusActions, SweepStart};
use crate::capture::{self, MappingRecord};
use crate::data;
use crate::hardware::{CameraApi, Capture, CaptureMode};
use crate::smart_scan::Zip;

/// The folder, in the data folder, that sequences are saved in.
pub const SEQUENCES: &str = "sequences";
/// The record's layout version.
const FORMAT: u32 = 1;
/// How many captures may wait to be saved.
const QUEUE: usize = 4;

/// How the latest sequence is going, or went.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LiveSequenceDetails {
    /// Its name, which is its folder's.
    pub name: String,
    /// How many time points its plan has.
    pub points: u64,
    /// How many images it has saved.
    pub image_count: u64,
    /// How many time points it has skipped.
    pub skipped: u64,
    /// How long it has run, in seconds.
    pub duration_seconds: f64,
    /// When its last time point is due, in seconds from its start.
    pub last_point_seconds: f64,
    /// The image it saved last, by its path in the data folder.
    pub latest_image: Option<String>,
    /// How it ended: `success`, `cancelled`, or the error. Empty while it runs.
    pub result: Option<String>,
}

/// The latest sequence's details, and when it started.
struct Latest {
    details: LiveSequenceDetails,
    started: Instant,
}

/// Runs timelapses: see the module's description.
#[derive(Thing)]
pub struct Sequence {
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    #[slot(default = "autofocus")]
    autofocus: Slot<Autofocus>,
    /// The time between captures, in seconds.
    #[setting(default = 30.0, ge = 0.5)]
    dt: Prop<f64>,
    /// How long the sequence runs, in seconds.
    #[setting(default = 600.0, ge = 0.0)]
    duration: Prop<f64>,
    /// Whether each capture follows an autofocus.
    #[setting(default = false)]
    autofocus_enabled: Prop<bool>,
    /// How far each autofocus sweeps, in steps.
    #[setting(default = 2000, ge = 100)]
    autofocus_dz: Prop<i64>,
    /// Held while a sequence runs, so only one does.
    running: tokio::sync::Mutex<()>,
    /// The sequence running, by name.
    current: Mutex<Option<String>>,
    /// The latest sequence, which its save queue updates too.
    latest: Arc<Mutex<Option<Latest>>>,
}

#[thing_impl]
impl Sequence {
    /// The latest sequence's details, while it runs and after it ends.
    #[property]
    async fn latest_sequence_live_details(&self) -> Option<LiveSequenceDetails> {
        let latest = self.latest.lock().expect("not poisoned");
        latest.as_ref().map(|latest| {
            let mut details = latest.details.clone();
            if details.result.is_none() {
                details.duration_seconds = latest.started.elapsed().as_secs_f64();
            }
            details
        })
    }

    /// Runs a timelapse into a new folder in `sequences/`, called `name`, or
    /// by the time without one, with `-2`, `-3`, … if that's taken: a capture
    /// every `dt` seconds for `duration`, each after an autofocus if
    /// `autofocus_enabled`. Returns the folder's name.
    #[action]
    async fn run_sequence(
        &self,
        ctx: ActionCtx,
        #[param(default)] name: String,
    ) -> Result<String, ActionError> {
        let Ok(_running) = self.running.try_lock() else {
            return Err(ActionError::handled("a sequence is already running"));
        };
        let focus = self
            .autofocus_enabled
            .get()
            .then(|| self.autofocus_dz.get());
        let (dt, duration) = (self.dt.get(), self.duration.get());
        let plan = SequencePlan::timelapse(
            Duration::from_secs_f64(dt),
            Duration::from_secs_f64(duration),
            focus,
        );
        plan.check().map_err(ActionError::handled)?;
        let sequences = data::data_folder(ctx.server()).join(SEQUENCES);
        let (name, folder) = data::new_folder(&sequences, &name, "sequence")?;
        *self.current.lock().expect("not poisoned") = Some(name.clone());
        let mut record = json!({
            "format": FORMAT,
            "name": name,
            "plan": {
                "dt": dt,
                "duration": duration,
                "autofocus_enabled": focus.is_some(),
                "autofocus_dz": self.autofocus_dz.get(),
            },
            "started": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            "duration_seconds": 0.0,
            "image_count": 0,
            "skipped": 0,
            "result": null,
        });
        write_record(&folder, &record)?;
        let started = Instant::now();
        let points = plan.points();
        *self.latest.lock().expect("not poisoned") = Some(Latest {
            details: LiveSequenceDetails {
                name: name.clone(),
                points,
                image_count: 0,
                skipped: 0,
                duration_seconds: 0.0,
                last_point_seconds: plan.at(points - 1).as_secs_f64(),
                latest_image: None,
                result: None,
            },
            started,
        });
        let images_at = format!("{SEQUENCES}/{name}");
        let mapping = capture::mapping(ctx.server());
        let saver = Saver::start(folder.clone(), images_at, mapping, Arc::clone(&self.latest));
        let outcome = self.run(&ctx, &plan, started, &saver).await;
        let outcome = outcome.and(saver.finish().await);
        let result = match &outcome {
            Ok(()) => "success".to_owned(),
            Err(error) if error.is_cancelled() => "cancelled".to_owned(),
            Err(error) => format!("error: {error}"),
        };
        let mut skipped = 0;
        self.update(|details| {
            details.duration_seconds = started.elapsed().as_secs_f64();
            details.result = Some(result.clone());
            skipped = details.skipped;
        });
        record["duration_seconds"] = json!(started.elapsed().as_secs_f64());
        record["image_count"] = json!(images(&folder).len());
        record["skipped"] = json!(skipped);
        record["result"] = json!(result);
        *self.current.lock().expect("not poisoned") = None;
        write_record(&folder, &record)?;
        outcome.map(|()| name)
    }

    /// The sequence `name` as a ZIP archive: its images with their sidecars,
    /// and its record.
    #[action(blocking, global_lock = false)]
    fn download_zip(&self, server: Server, name: String) -> Result<Blob<Zip>, ActionError> {
        let sequences = data::data_folder(&server).join(SEQUENCES);
        let folder = data::resolve(&sequences, &name).ok().filter(|f| f.is_dir());
        let plain = !name.contains(['/', '\\']);
        let Some(folder) = folder.filter(|_| plain) else {
            return Err(ActionError::handled(format!(
                "there's no sequence `{name}`"
            )));
        };
        let mut files: Vec<(String, PathBuf)> = (images(&folder).into_iter())
            .flat_map(|image| [image.clone(), image.replace(".jpeg", ".json")])
            .chain(["sequence.json".to_owned()])
            .map(|file| (file.clone(), folder.join(file)))
            .filter(|(_, path)| path.is_file())
            .collect();
        files.sort();
        let archive = data::zip(&files, Cursor::new(Vec::new())).map_err(ActionError::handled)?;
        Ok(Blob::from_bytes(archive.into_inner()))
    }
}

impl Sequence {
    /// The name of the sequence running, if one is.
    pub fn running(&self) -> Option<String> {
        self.current.lock().expect("not poisoned").clone()
    }

    /// Changes the latest sequence's details.
    fn update(&self, change: impl FnOnce(&mut LiveSequenceDetails)) {
        if let Some(latest) = self.latest.lock().expect("not poisoned").as_mut() {
            change(&mut latest.details);
        }
    }

    /// Runs `plan`'s time points from `started` until it's done, counting
    /// the points skipped.
    async fn run(
        &self,
        ctx: &ActionCtx,
        plan: &SequencePlan,
        started: Instant,
        saver: &Saver,
    ) -> Result<(), ActionError> {
        let mut next = 0;
        loop {
            let late = match plan.due(started.elapsed(), next) {
                Due::Wait(wait) => {
                    ctx.sleep(wait).await?;
                    continue;
                }
                Due::Done { skipped: late } => {
                    report(late);
                    self.update(|details| details.skipped += late);
                    return Ok(());
                }
                Due::Run {
                    index,
                    skipped: late,
                } => {
                    next = index + 1;
                    self.point(ctx, plan, index, saver).await?;
                    late
                }
            };
            report(late);
            self.update(|details| details.skipped += late);
        }
    }

    /// Runs `plan`'s steps for time point `index`.
    async fn point(
        &self,
        ctx: &ActionCtx,
        plan: &SequencePlan,
        index: u64,
        saver: &Saver,
    ) -> Result<(), ActionError> {
        ctx.check_cancelled()?;
        for step in &plan.steps {
            match *step {
                Step::Autofocus { dz } => {
                    let focused = self.autofocus.fast_autofocus(dz, SweepStart::Centre).await;
                    match focused {
                        Err(error) if error.is_cancelled() => return Err(error),
                        Err(error) => {
                            tracing::warn!("Autofocus failed, so capturing anyway: {error}")
                        }
                        Ok(_) => {}
                    }
                }
                Step::Capture => {
                    let capture = self.camera.capture_to_memory(CaptureMode::Standard).await?;
                    saver.save(index, capture).await?;
                }
                Step::ZStack { .. } => unreachable!("checked before running"),
            }
        }
        Ok(())
    }
}

/// Logs that `late` time points were skipped, if any were.
fn report(late: u64) {
    let points = if late == 1 { "point" } else { "points" };
    if late > 0 {
        tracing::warn!("Skipped {late} time {points}: the steps took longer than the interval");
    }
}

/// The images in the sequence folder `folder`, by name, in order.
pub fn images(folder: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(folder).into_iter().flatten().flatten();
    let mut images: Vec<String> = (entries
        .filter_map(|entry| entry.file_name().into_string().ok()))
    .filter(|file| file.starts_with("image_") && file.ends_with(".jpeg"))
    .collect();
    images.sort();
    images
}

/// Writes a sequence's record, `sequence.json`, in `folder`.
fn write_record(folder: &Path, record: &Value) -> Result<(), ActionError> {
    let text = serde_json::to_vec_pretty(record).map_err(ActionError::handled)?;
    std::fs::write(folder.join("sequence.json"), text).map_err(ActionError::handled)
}

/// Saves captures, in the order given, beside the schedule: at most `QUEUE`
/// wait, and a capture that finds the queue full waits for room. Each one
/// saved is counted in the latest sequence's details.
struct Saver {
    sender: mpsc::Sender<(u64, Capture)>,
    task: JoinHandle<Result<(), ActionError>>,
}

impl Saver {
    /// Saves into `folder`, which is `images_at` in the data folder.
    fn start(
        folder: PathBuf,
        images_at: String,
        mapping: Option<MappingRecord>,
        latest: Arc<Mutex<Option<Latest>>>,
    ) -> Saver {
        let (sender, mut receiver) = mpsc::channel::<(u64, Capture)>(QUEUE);
        let task = tokio::spawn(async move {
            while let Some((index, capture)) = receiver.recv().await {
                let file = format!("image_{index:05}.jpeg");
                let path = folder.join(&file);
                let mapping = mapping.clone();
                let written =
                    tokio::task::spawn_blocking(move || capture::save_to(&path, &capture, mapping));
                written.await.map_err(ActionError::handled)??;
                if let Some(latest) = latest.lock().expect("not poisoned").as_mut() {
                    latest.details.image_count += 1;
                    latest.details.latest_image = Some(format!("{images_at}/{file}"));
                }
            }
            Ok(())
        });
        Saver { sender, task }
    }

    /// Queues `capture`, time point `index`'s, to be saved.
    async fn save(&self, index: u64, capture: Capture) -> Result<(), ActionError> {
        (self.sender.send((index, capture)).await)
            .map_err(|_| ActionError::handled("the captures can't be saved"))
    }

    /// Saves what's queued.
    async fn finish(self) -> Result<(), ActionError> {
        drop(self.sender);
        self.task.await.map_err(ActionError::handled)?
    }
}
