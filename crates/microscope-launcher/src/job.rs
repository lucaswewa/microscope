//! The Windows Job Object that ends the server with the launcher, however
//! the launcher ends (ADR-0033).

use tokio::process::Child;
use win32job::{ExtendedLimitInfo, Job};

/// A job that holds `child` and ends it once the job is dropped, or the
/// launcher ends.
pub fn contain(child: &Child) -> Result<Job, String> {
    let mut limits = ExtendedLimitInfo::new();
    limits.limit_kill_on_job_close();
    let job = Job::create_with_limit_info(&limits).map_err(|e| format!("no job object: {e}"))?;
    let handle = child.raw_handle().ok_or("the server has already exited")?;
    (job.assign_process(handle as isize)).map_err(|e| format!("no job object: {e}"))?;
    Ok(job)
}
