//! Basler's pylon, through its C API (PylonC).
//!
//! pylon's installer puts the library in `pylon\Runtime\x64` on Windows and
//! `/opt/pylon/lib` on Linux. `MICROSCOPE_PYLON_LIBRARY` names it elsewhere.
//!
//! - **[`Pylon::cameras`]** lists the cameras, by serial number and model.
//! - **[`Pylon::open`]** opens one, as a [`PylonCamera`]: its features, read
//!   and written as text by their GenICam names, and its frames, grabbed
//!   continuously on a thread of their own ([`PylonCamera::grab`]).
//! - **pylon stays initialised** while a camera is open, or while it counts
//!   them.

use std::ffi::{CStr, CString, OsString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crate::{SdkError, Search, folder, one_at_a_time};

const SDK: &str = "pylon";
/// Buffers queued for grabbing.
const BUFFERS: usize = 4;
/// How long the grab waits for a frame before looking again.
const WAIT_MS: u32 = 500;
/// `PYLONC_ACCESS_MODE_CONTROL | PYLONC_ACCESS_MODE_STREAM`.
const CONTROL_AND_STREAM: i32 = 1 | 2;
/// `EPylonGrabStatus`'s `Grabbed`.
const GRABBED: i32 = 2;

/// A handle of pylon's (`PYLON_DEVICE_HANDLE` and the like).
type Handle = *mut c_void;

/// `PylonGrabResult_t`, field for field, aligned as pylon packs it (8).
#[repr(C)]
struct GrabResult {
    context: *const c_void,
    buffer: Handle,
    data: *const c_void,
    status: i32,
    payload_type: i32,
    pixel_type: u32,
    time_stamp: u64,
    size_x: i32,
    size_y: i32,
    offset_x: i32,
    offset_y: i32,
    padding_x: i32,
    padding_y: i32,
    payload_size: u64,
    error_code: u32,
    block_id: u64,
}

macro_rules! functions {
    ($($field:ident: $name:literal fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        /// pylon's functions, as `PylonC.h` and `GenApiC.h` declare them.
        struct Functions {
            $($field: unsafe extern "system" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Functions {
            fn load(loaded: &crate::Loaded) -> Result<Functions, SdkError> {
                Ok(Functions {
                    // SAFETY: each type is its function's in pylon's headers,
                    // with GENAPIC_RESULT an HRESULT (32 bits), size_t a
                    // usize, _Bool a bool, and handles pointers.
                    // `extern "system"` is PYLONC_CC: `__stdcall` on
                    // Windows, C elsewhere.
                    $($field: unsafe { loaded.function($name) }?,)*
                })
            }
        }
    };
}

functions! {
    initialize: "PylonInitialize" fn() -> i32;
    terminate: "PylonTerminate" fn(bool) -> i32;
    enumerate: "PylonEnumerateDevices" fn(*mut usize) -> i32;
    info: "PylonGetDeviceInfoHandle" fn(usize, *mut Handle) -> i32;
    info_value: "PylonDeviceInfoGetPropertyValueByName" fn(Handle, *const c_char, *mut c_char, *mut usize) -> i32;
    create: "PylonCreateDeviceByIndex" fn(usize, *mut Handle) -> i32;
    open: "PylonDeviceOpen" fn(Handle, i32) -> i32;
    close: "PylonDeviceClose" fn(Handle) -> i32;
    destroy: "PylonDestroyDevice" fn(Handle) -> i32;
    available: "PylonDeviceFeatureIsAvailable" fn(Handle, *const c_char) -> bool;
    to_string: "PylonDeviceFeatureToString" fn(Handle, *const c_char, *mut c_char, *mut usize) -> i32;
    from_string: "PylonDeviceFeatureFromString" fn(Handle, *const c_char, *const c_char) -> i32;
    execute: "PylonDeviceExecuteCommandFeature" fn(Handle, *const c_char) -> i32;
    node_map: "PylonDeviceGetNodeMap" fn(Handle, *mut Handle) -> i32;
    node: "GenApiNodeMapGetNode" fn(Handle, *const c_char, *mut Handle) -> i32;
    entries: "GenApiEnumerationGetNumEntries" fn(Handle, *mut usize) -> i32;
    entry: "GenApiEnumerationGetEntryByIndex" fn(Handle, usize, *mut Handle) -> i32;
    symbolic: "GenApiEnumerationEntryGetSymbolic" fn(Handle, *mut c_char, *mut usize) -> i32;
    node_available: "GenApiNodeIsAvailable" fn(Handle, *mut bool) -> i32;
    last_error: "GenApiGetLastErrorMessage" fn(*mut c_char, *mut usize) -> i32;
    grabber: "PylonDeviceGetStreamGrabber" fn(Handle, usize, *mut Handle) -> i32;
    grabber_open: "PylonStreamGrabberOpen" fn(Handle) -> i32;
    grabber_close: "PylonStreamGrabberClose" fn(Handle) -> i32;
    wait_object: "PylonStreamGrabberGetWaitObject" fn(Handle, *mut Handle) -> i32;
    payload_size: "PylonStreamGrabberGetPayloadSize" fn(Handle, Handle, *mut usize) -> i32;
    max_buffers: "PylonStreamGrabberSetMaxNumBuffer" fn(Handle, usize) -> i32;
    max_buffer_size: "PylonStreamGrabberSetMaxBufferSize" fn(Handle, usize) -> i32;
    prepare: "PylonStreamGrabberPrepareGrab" fn(Handle) -> i32;
    register: "PylonStreamGrabberRegisterBuffer" fn(Handle, *mut c_void, usize, *mut Handle) -> i32;
    deregister: "PylonStreamGrabberDeregisterBuffer" fn(Handle, Handle) -> i32;
    queue: "PylonStreamGrabberQueueBuffer" fn(Handle, Handle, *const c_void) -> i32;
    start_streaming: "PylonStreamGrabberStartStreamingIfMandatory" fn(Handle) -> i32;
    stop_streaming: "PylonStreamGrabberStopStreamingIfMandatory" fn(Handle) -> i32;
    retrieve: "PylonStreamGrabberRetrieveResult" fn(Handle, *mut GrabResult, *mut bool) -> i32;
    cancel: "PylonStreamGrabberCancelGrab" fn(Handle) -> i32;
    finish: "PylonStreamGrabberFinishGrab" fn(Handle) -> i32;
    wait: "PylonWaitObjectWait" fn(Handle, u32, *mut bool) -> i32;
}

/// pylon's C library, loaded.
pub struct Pylon {
    f: Functions,
    path: PathBuf,
    // Kept loaded while its functions are.
    _library: libloading::Library,
}

/// A camera pylon finds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraInfo {
    /// Its serial number, which a configuration names it by.
    pub serial_number: String,
    /// Its model, such as `acA1920-40uc`.
    pub model: String,
}

impl Pylon {
    /// Finds and loads pylon's C library.
    pub fn load() -> Result<Arc<Pylon>, SdkError> {
        Pylon::load_from(|name| std::env::var_os(name)).map(Arc::new)
    }

    fn load_from(var: impl Fn(&str) -> Option<OsString>) -> Result<Pylon, SdkError> {
        let loaded = search(&var).load(var)?;
        let f = Functions::load(&loaded)?;
        let (path, _library) = (loaded.path, loaded.library);
        Ok(Pylon { f, path, _library })
    }

    /// The library loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The cameras pylon finds.
    pub fn cameras(self: &Arc<Self>) -> Result<Vec<CameraInfo>, SdkError> {
        let _session = Session::start(self)?;
        let mut count = 0;
        // SAFETY: pylon is initialised, and `count` is a size_t to write.
        self.check("PylonEnumerateDevices", unsafe {
            (self.f.enumerate)(&mut count)
        })?;
        (0..count).map(|index| self.info(index)).collect()
    }

    /// The camera at `index` of those last enumerated.
    fn info(&self, index: usize) -> Result<CameraInfo, SdkError> {
        let mut info = std::ptr::null_mut();
        // SAFETY: `index` is below the count enumerated; `info` is a handle
        // to write.
        self.check("PylonGetDeviceInfoHandle", unsafe {
            (self.f.info)(index, &mut info)
        })?;
        let value = |name: &str| {
            let name = c(name);
            // SAFETY: `info` is the handle pylon gave, `name` a C string,
            // and `text` writes no more than the length it's given.
            self.text(
                "PylonDeviceInfoGetPropertyValueByName",
                |buffer, length| unsafe {
                    (self.f.info_value)(info, name.as_ptr(), buffer, length)
                },
            )
        };
        let (serial_number, model) = (value("SerialNumber")?, value("ModelName")?);
        Ok(CameraInfo {
            serial_number,
            model,
        })
    }

    /// Opens the camera with `serial_number`, or the first pylon finds, to
    /// control it and grab from it.
    pub fn open(self: &Arc<Self>, serial_number: Option<&str>) -> Result<PylonCamera, SdkError> {
        let session = Session::start(self)?;
        let cameras = self.cameras()?;
        let found = cameras
            .iter()
            .position(|camera| serial_number.is_none_or(|serial| camera.serial_number == serial));
        let Some(index) = found else {
            let failed = "PylonCreateDeviceByIndex";
            let detail = match serial_number {
                Some(serial) => format!("no camera has the serial number {serial}"),
                None => "there's no camera".to_owned(),
            };
            return Err(SdkError::Failed {
                sdk: SDK,
                function: failed,
                code: 0,
                detail,
            });
        };
        let mut handle = std::ptr::null_mut();
        // SAFETY: `index` is a camera's, of those just enumerated.
        self.check("PylonCreateDeviceByIndex", unsafe {
            (self.f.create)(index, &mut handle)
        })?;
        let device = Device { session, handle };
        // SAFETY: the device was just created, and isn't open.
        let opened = unsafe { (self.f.open)(device.handle, CONTROL_AND_STREAM) };
        self.check("PylonDeviceOpen", opened)?;
        let info = cameras[index].clone();
        Ok(PylonCamera {
            device: Arc::new(device),
            info,
        })
    }

    /// `Ok` if `code`, from `function`, is 0, or else an error with pylon's
    /// own message.
    fn check(&self, function: &'static str, code: i32) -> Result<(), SdkError> {
        if code == 0 {
            return Ok(());
        }
        // SAFETY: the message is written into the buffer, at most its length.
        let detail = self.text("", |buffer, length| unsafe {
            (self.f.last_error)(buffer, length)
        });
        Err(SdkError::Failed {
            sdk: SDK,
            function,
            code,
            detail: detail.unwrap_or_default(),
        })
    }

    /// The text `get` writes, given a buffer and its length.
    fn text(
        &self,
        function: &'static str,
        get: impl Fn(*mut c_char, *mut usize) -> i32,
    ) -> Result<String, SdkError> {
        let mut buffer = vec![0 as c_char; 1024];
        let mut length = buffer.len();
        let code = get(buffer.as_mut_ptr(), &mut length);
        if code != 0 && !function.is_empty() {
            self.check(function, code)?;
        }
        // SAFETY: the buffer was zeroed, and pylon ends what it writes with a
        // null.
        let text = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        Ok(text.to_string_lossy().into_owned())
    }
}

/// pylon initialised, until it's dropped.
struct Session(Arc<Pylon>);

impl Session {
    fn start(pylon: &Arc<Pylon>) -> Result<Session, SdkError> {
        let _starting = one_at_a_time();
        // SAFETY: it takes no arguments; the session's drop matches it with
        // PylonTerminate, as pylon counts them.
        pylon.check("PylonInitialize", unsafe { (pylon.f.initialize)() })?;
        Ok(Session(Arc::clone(pylon)))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _stopping = one_at_a_time();
        // SAFETY: it matches the session's PylonInitialize; logging stays
        // up, for the next session.
        let _ = unsafe { (self.0.f.terminate)(false) };
    }
}

/// An open camera's handle, closed when the last of the camera and its
/// grab has gone.
struct Device {
    session: Session,
    handle: Handle,
}

// SAFETY: pylon's handles may be used from any thread. The grab's thread
// uses the device's stream grabber, which nothing else does, and asks the
// device only to start and stop acquiring.
unsafe impl Send for Device {}
// SAFETY: as above: pylon's device functions may be called from several
// threads.
unsafe impl Sync for Device {}

impl Device {
    fn pylon(&self) -> &Pylon {
        &self.session.0
    }

    fn execute(&self, feature: &str) -> Result<(), SdkError> {
        let name = c(feature);
        // SAFETY: the device is open, and `name` a C string.
        let code = unsafe { (self.pylon().f.execute)(self.handle, name.as_ptr()) };
        self.pylon().check("PylonDeviceExecuteCommandFeature", code)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let f = &self.pylon().f;
        // SAFETY: the device is open, and nothing uses it any more.
        let _ = unsafe { (f.close)(self.handle) };
        // SAFETY: it's closed, and destroyed once.
        let _ = unsafe { (f.destroy)(self.handle) };
    }
}

/// An open Basler camera.
pub struct PylonCamera {
    device: Arc<Device>,
    info: CameraInfo,
}

/// A frame as the camera sent it.
#[derive(Debug, Clone, Copy)]
pub struct Grabbed<'a> {
    /// Its width, in pixels.
    pub width: u32,
    /// Its height, in pixels.
    pub height: u32,
    /// Its pixel format's GenICam name, such as `Mono8`.
    pub pixel_format: &'a str,
    /// Its pixels, row by row, with no padding.
    pub data: &'a [u8],
}

impl PylonCamera {
    /// Which camera it is.
    pub fn info(&self) -> &CameraInfo {
        &self.info
    }

    /// Whether the camera has the feature `name`, and it's available now.
    pub fn has(&self, name: &str) -> bool {
        let name = c(name);
        // SAFETY: the device is open, and `name` a C string.
        unsafe { (self.pylon().f.available)(self.device.handle, name.as_ptr()) }
    }

    /// The feature `name`'s value, as text.
    pub fn get(&self, name: &str) -> Result<String, SdkError> {
        let (pylon, name) = (self.pylon(), c(name));
        // SAFETY: the device is open, `name` a C string, and `text` writes
        // no more than the length it's given.
        pylon.text("PylonDeviceFeatureToString", |buffer, length| unsafe {
            (pylon.f.to_string)(self.device.handle, name.as_ptr(), buffer, length)
        })
    }

    /// Sets the feature `name` from `value`, as text: a number, or one of
    /// its choices.
    pub fn set(&self, name: &str, value: &str) -> Result<(), SdkError> {
        let (name, value) = (c(name), c(value));
        // SAFETY: the device is open, and both are C strings.
        let code = unsafe {
            (self.pylon().f.from_string)(self.device.handle, name.as_ptr(), value.as_ptr())
        };
        self.pylon().check("PylonDeviceFeatureFromString", code)
    }

    /// Runs the command feature `name`.
    pub fn execute(&self, name: &str) -> Result<(), SdkError> {
        self.device.execute(name)
    }

    /// The choices of the enumeration feature `name` available now.
    pub fn choices(&self, name: &str) -> Result<Vec<String>, SdkError> {
        let (pylon, f) = (self.pylon(), &self.pylon().f);
        let (mut map, mut node, mut count) = (std::ptr::null_mut(), std::ptr::null_mut(), 0);
        // SAFETY: the device is open; `map` is a handle to write.
        pylon.check("PylonDeviceGetNodeMap", unsafe {
            (f.node_map)(self.device.handle, &mut map)
        })?;
        let name = c(name);
        // SAFETY: `map` is the device's node map, `name` a C string.
        pylon.check("GenApiNodeMapGetNode", unsafe {
            (f.node)(map, name.as_ptr(), &mut node)
        })?;
        // SAFETY: `node` is the enumeration's; `count` a size_t to write.
        pylon.check("GenApiEnumerationGetNumEntries", unsafe {
            (f.entries)(node, &mut count)
        })?;
        let mut choices = Vec::new();
        for index in 0..count {
            let (mut entry, mut available) = (std::ptr::null_mut(), false);
            // SAFETY: `index` is below the count; `entry` a handle to write.
            let code = unsafe { (f.entry)(node, index, &mut entry) };
            pylon.check("GenApiEnumerationGetEntryByIndex", code)?;
            // SAFETY: `entry` is the entry's handle; `available` a _Bool.
            let code = unsafe { (f.node_available)(entry, &mut available) };
            pylon.check("GenApiNodeIsAvailable", code)?;
            if available {
                // SAFETY: `entry` is an entry's handle, and `text` writes no
                // more than the length it's given.
                let symbolic = pylon.text(
                    "GenApiEnumerationEntryGetSymbolic",
                    |buffer, length| unsafe { (f.symbolic)(entry, buffer, length) },
                );
                choices.push(symbolic?);
            }
        }
        Ok(choices)
    }

    /// Grabs continuously on a thread of its own, giving each frame to
    /// `deliver`, until the [`Grabbing`] returned is dropped. The camera's
    /// pixel format, width and height can't change meanwhile.
    pub fn grab(
        &self,
        deliver: impl FnMut(Grabbed<'_>) + Send + 'static,
    ) -> Result<Grabbing, SdkError> {
        // Frame after frame, as fast as the exposure allows.
        for (feature, value) in [("AcquisitionMode", "Continuous"), ("TriggerMode", "Off")] {
            if self.has(feature) {
                self.set(feature, value)?;
            }
        }
        let pixel_format = self.get("PixelFormat")?;
        let grab = Grab::prepare(&self.device)?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name("pylon grab".to_owned())
            .spawn({
                let stop = Arc::clone(&stop);
                move || grab.run(&stop, &pixel_format, deliver)
            });
        let thread = thread.map_err(|error| SdkError::Failed {
            sdk: SDK,
            function: "a grab thread",
            code: 0,
            detail: error.to_string(),
        })?;
        let thread = Some(thread);
        Ok(Grabbing { stop, thread })
    }

    fn pylon(&self) -> &Pylon {
        self.device.pylon()
    }
}

/// A camera grabbing, until it's dropped.
pub struct Grabbing {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), SdkError>>>,
}

impl Grabbing {
    /// Stops grabbing, and returns why the grab had stopped, if it had.
    pub fn stop(mut self) -> Result<(), SdkError> {
        self.finish()
    }

    fn finish(&mut self) -> Result<(), SdkError> {
        self.stop.store(true, Ordering::Relaxed);
        let thread = self.thread.take();
        thread.map_or(Ok(()), |thread| thread.join().unwrap_or(Ok(())))
    }
}

impl Drop for Grabbing {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// A stream grabber, ready to grab into its buffers.
struct Grab {
    device: Arc<Device>,
    grabber: Handle,
    wait: Handle,
    buffers: Vec<(Vec<u8>, Handle)>,
}

// SAFETY: the grabber, its wait object and its buffers are used only by the
// grab's thread, once it's moved there.
unsafe impl Send for Grab {}

impl Grab {
    fn prepare(device: &Arc<Device>) -> Result<Grab, SdkError> {
        let (pylon, f, handle) = (device.pylon(), &device.pylon().f, device.handle);
        let (mut grabber, mut wait, mut size) = (std::ptr::null_mut(), std::ptr::null_mut(), 0);
        // SAFETY: the device is open; channel 0 is its first stream.
        pylon.check("PylonDeviceGetStreamGrabber", unsafe {
            (f.grabber)(handle, 0, &mut grabber)
        })?;
        // SAFETY: `grabber` is the device's, not yet open.
        pylon.check("PylonStreamGrabberOpen", unsafe {
            (f.grabber_open)(grabber)
        })?;
        // SAFETY: the grabber is open; `wait` is a handle to write.
        pylon.check("PylonStreamGrabberGetWaitObject", unsafe {
            (f.wait_object)(grabber, &mut wait)
        })?;
        // SAFETY: as above; `size` is a size_t to write.
        let code = unsafe { (f.payload_size)(handle, grabber, &mut size) };
        pylon.check("PylonStreamGrabberGetPayloadSize", code)?;
        // SAFETY: the grabber is open, and not yet prepared.
        pylon.check("PylonStreamGrabberSetMaxNumBuffer", unsafe {
            (f.max_buffers)(grabber, BUFFERS)
        })?;
        // SAFETY: as above.
        let code = unsafe { (f.max_buffer_size)(grabber, size) };
        pylon.check("PylonStreamGrabberSetMaxBufferSize", code)?;
        // SAFETY: as above.
        pylon.check("PylonStreamGrabberPrepareGrab", unsafe {
            (f.prepare)(grabber)
        })?;
        let mut buffers = Vec::new();
        for index in 0..BUFFERS {
            let mut buffer = vec![0u8; size];
            let mut registered = std::ptr::null_mut();
            // SAFETY: the buffer lives, unmoved, in `buffers` until it's
            // deregistered; its length is `size`.
            let code =
                unsafe { (f.register)(grabber, buffer.as_mut_ptr().cast(), size, &mut registered) };
            pylon.check("PylonStreamGrabberRegisterBuffer", code)?;
            // SAFETY: it's registered; the context is the buffer's index.
            let code = unsafe { (f.queue)(grabber, registered, index as *const c_void) };
            pylon.check("PylonStreamGrabberQueueBuffer", code)?;
            buffers.push((buffer, registered));
        }
        // SAFETY: the grabber is prepared, with its buffers queued.
        let code = unsafe { (f.start_streaming)(grabber) };
        pylon.check("PylonStreamGrabberStartStreamingIfMandatory", code)?;
        device.execute("AcquisitionStart")?;
        let device = Arc::clone(device);
        Ok(Grab {
            device,
            grabber,
            wait,
            buffers,
        })
    }

    /// Delivers each frame until `stop`, then stops, and returns why it
    /// stopped early, if it did.
    fn run(
        self,
        stop: &AtomicBool,
        pixel_format: &str,
        mut deliver: impl FnMut(Grabbed<'_>),
    ) -> Result<(), SdkError> {
        let (pylon, f) = (self.device.pylon(), &self.device.pylon().f);
        let mut packed = Vec::new();
        let mut outcome = Ok(());
        while !stop.load(Ordering::Relaxed) {
            let mut ready = false;
            // SAFETY: `wait` is the grabber's wait object; `ready` a _Bool.
            let code = unsafe { (f.wait)(self.wait, WAIT_MS, &mut ready) };
            if let Err(error) = pylon.check("PylonWaitObjectWait", code) {
                outcome = Err(error);
                break;
            }
            if !ready {
                continue;
            }
            // SAFETY: a GrabResult is plain data; zero is a valid value.
            let mut result: GrabResult = unsafe { std::mem::zeroed() };
            // SAFETY: the grabber is grabbing; `result` and `ready` are to
            // write.
            let code = unsafe { (f.retrieve)(self.grabber, &mut result, &mut ready) };
            if let Err(error) = pylon.check("PylonStreamGrabberRetrieveResult", code) {
                outcome = Err(error);
                break;
            }
            if !ready {
                continue;
            }
            if result.status == GRABBED {
                let (width, height) = (result.size_x.max(0) as u32, result.size_y.max(0) as u32);
                let row = width as usize * bytes_per_pixel(pixel_format);
                let stride = row + result.padding_x.max(0) as usize;
                let size = (stride * height as usize).min(result.payload_size as usize);
                // SAFETY: pylon filled the result's buffer, one of ours,
                // with `payload_size` bytes, and it isn't requeued until
                // `deliver` is done with it.
                let raw = unsafe { std::slice::from_raw_parts(result.data.cast::<u8>(), size) };
                let data = if stride == row {
                    raw
                } else {
                    packed.clear();
                    raw.chunks(stride)
                        .for_each(|line| packed.extend_from_slice(&line[..row.min(line.len())]));
                    &packed
                };
                deliver(Grabbed {
                    width,
                    height,
                    pixel_format,
                    data,
                });
            }
            // SAFETY: the result's buffer is one of ours, done with.
            let code = unsafe { (f.queue)(self.grabber, result.buffer, result.context) };
            if let Err(error) = pylon.check("PylonStreamGrabberQueueBuffer", code) {
                outcome = Err(error);
                break;
            }
        }
        outcome
    }
}

impl Drop for Grab {
    fn drop(&mut self) {
        let f = &self.device.pylon().f;
        let _ = self.device.execute("AcquisitionStop");
        // SAFETY: the grabber was prepared; cancelling returns every queued
        // buffer.
        let _ = unsafe { (f.cancel)(self.grabber) };
        loop {
            let mut ready = false;
            // SAFETY: a GrabResult is plain data; zero is a valid value.
            let mut result: GrabResult = unsafe { std::mem::zeroed() };
            // SAFETY: as in `run`, emptying the grabber's results.
            let code = unsafe { (f.retrieve)(self.grabber, &mut result, &mut ready) };
            if code != 0 || !ready {
                break;
            }
        }
        // SAFETY: the grabber is cancelled and empty.
        let _ = unsafe { (f.stop_streaming)(self.grabber) };
        // SAFETY: as above.
        let _ = unsafe { (f.finish)(self.grabber) };
        for (_, registered) in &self.buffers {
            // SAFETY: each buffer was registered once, and is no longer
            // queued.
            let _ = unsafe { (f.deregister)(self.grabber, *registered) };
        }
        // SAFETY: the grabber is finished, its buffers deregistered.
        let _ = unsafe { (f.grabber_close)(self.grabber) };
    }
}

/// The bytes a pixel takes in the format GenICam calls `name`, for the
/// formats a camera sends unpacked: 1 for 8 bits, 2 for 10 to 16, 3 for RGB.
fn bytes_per_pixel(name: &str) -> usize {
    let bits: usize = name
        .trim_start_matches(char::is_alphabetic)
        .parse()
        .unwrap_or(8);
    let colour = name.starts_with("RGB") || name.starts_with("BGR");
    if colour { 3 } else { bits.div_ceil(8) }
}

/// `text` as a C string.
fn c(text: &str) -> CString {
    CString::new(text.replace('\0', "")).expect("no nulls left")
}

/// Where pylon's installer puts its C library, then its name for the
/// system's search.
fn search(var: &impl Fn(&str) -> Option<OsString>) -> Search {
    let mut places = Vec::new();
    if cfg!(windows) {
        let name = "PylonC_v10.dll";
        let runtime = |pylon: &Path| pylon.join("Runtime").join("x64").join(name);
        // PYLON_DEV_DIR is the `Development` folder in pylon's.
        let development = folder(var, "PYLON_DEV_DIR");
        if let Some(pylon) = development.as_deref().and_then(Path::parent) {
            places.push(runtime(pylon));
        }
        if let Some(programs) = folder(var, "ProgramFiles") {
            places.push(runtime(&programs.join("Basler").join("pylon")));
        }
        places.push(PathBuf::from(name));
    } else {
        let name = "libpylonc.so";
        if let Some(root) = folder(var, "PYLON_ROOT") {
            places.push(root.join("lib").join(name));
        }
        places.push(PathBuf::from("/opt/pylon/lib").join(name));
        places.push(PathBuf::from(name));
    }
    Search {
        sdk: SDK,
        variable: "MICROSCOPE_PYLON_LIBRARY",
        install: "https://www.baslerweb.com/pylon",
        places,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn pylon_is_looked_for_where_its_installer_puts_it() {
        let var = |name: &str| match name {
            "PYLON_DEV_DIR" => Some(OsString::from("D:/pylon/Development/")),
            "PYLON_ROOT" => Some(OsString::from("/home/me/pylon")),
            _ => None,
        };
        let places = search(&var).places;
        if cfg!(windows) {
            let runtime = Path::new("D:/pylon/Runtime/x64");
            assert_eq!(places[0], runtime.join("PylonC_v10.dll"));
        } else {
            assert_eq!(places[0], Path::new("/home/me/pylon/lib/libpylonc.so"));
            assert_eq!(places[1], Path::new("/opt/pylon/lib/libpylonc.so"));
        }
        assert!(places.last().expect("a name").parent() == Some(Path::new("")));
    }

    #[test]
    fn without_pylon_the_error_says_how_to_get_it() {
        let folder = tempfile::tempdir().expect("a folder");
        let missing = folder.path().join("PylonC_v10.dll");
        let var = |name: &str| (name == "MICROSCOPE_PYLON_LIBRARY").then(|| missing.clone().into());
        let message = Pylon::load_from(var).err().expect("not loaded").to_string();
        assert!(message.starts_with("pylon isn't installed"), "{message}");
        assert!(
            message.contains("https://www.baslerweb.com/pylon"),
            "{message}"
        );
    }

    #[test]
    fn a_pixel_takes_the_bytes_its_format_says() {
        assert_eq!(bytes_per_pixel("Mono8"), 1);
        assert_eq!(bytes_per_pixel("Mono12"), 2);
        assert_eq!(bytes_per_pixel("BayerRG16"), 2);
        assert_eq!(bytes_per_pixel("BGR8"), 3);
    }

    /// Where pylon is installed, such as on the owner's PC; skipped where
    /// it isn't, such as in CI.
    #[test]
    fn an_installed_pylon_lists_its_cameras() {
        let pylon = match Pylon::load() {
            Ok(pylon) => pylon,
            Err(error) => return eprintln!("skipped: {error}"),
        };
        let cameras = pylon.cameras().expect("listed");
        eprintln!("pylon at {} finds {cameras:?}", pylon.path().display());
    }

    /// Where pylon's camera emulator is on (`PYLON_CAMEMU=1`): a camera
    /// opens by its serial number, its features read and write, and it
    /// grabs frames of the size it says.
    #[test]
    fn the_camera_emulator_opens_and_grabs() {
        let Ok(pylon) = Pylon::load() else {
            return eprintln!("skipped: pylon isn't installed");
        };
        if std::env::var_os("PYLON_CAMEMU").is_none() {
            return eprintln!("skipped: set PYLON_CAMEMU=1 to try pylon's camera emulator");
        }
        let camera = pylon.open(Some("0815-0000")).expect("the emulator opens");
        assert_eq!(camera.info().model, "Emulation");
        assert!(camera.has("ExposureTime") && !camera.has("NoSuchFeature"));
        camera.set("ExposureTime", "5000").expect("set");
        assert_eq!(
            camera
                .get("ExposureTime")
                .expect("read")
                .parse::<f64>()
                .ok(),
            Some(5000.0)
        );
        assert!(
            camera
                .choices("PixelFormat")
                .expect("listed")
                .contains(&"Mono8".to_owned())
        );
        let refused = camera
            .set("ExposureTime", "fast")
            .expect_err("refused")
            .to_string();
        assert!(
            refused.starts_with("pylon's PylonDeviceFeatureFromString failed"),
            "{refused}"
        );
        let (width, height) = (
            camera.get("Width").expect("read"),
            camera.get("Height").expect("read"),
        );
        let frames = Arc::new(Mutex::new(Vec::new()));
        let grabbing = camera
            .grab({
                let frames = Arc::clone(&frames);
                move |frame| {
                    frames
                        .lock()
                        .expect("ok")
                        .push((frame.width, frame.height, frame.data.len()))
                }
            })
            .expect("grabbing");
        let deadline = Instant::now() + Duration::from_secs(10);
        while frames.lock().expect("ok").len() < 3 {
            if Instant::now() > deadline {
                panic!("no frames: the grab ended with {:?}", grabbing.stop());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        grabbing.stop().expect("stopped");
        let (w, h, bytes) = frames.lock().expect("ok")[0];
        assert_eq!((w.to_string(), h.to_string()), (width, height));
        assert_eq!(bytes, (w * h) as usize);
        assert!(pylon.open(Some("no-such-camera")).is_err());
    }
}
