//! Background jobs with progress and cancellation.
//!
//! Every long computation (sonogram tiles, CT search, ACF, resampling) runs through
//! [`spawn`], so the UI never blocks and the user can always cancel. This fixes SA's
//! "no progress indication, no panic!" behaviour.
//!
//! ```
//! use sn_core::job::{self, JobError};
//!
//! let handle = job::spawn("sum", |ctx| {
//!     ctx.set_total(100);
//!     let mut acc = 0_u64;
//!     for i in 0..100 {
//!         ctx.checkpoint()?;          // returns Err(Cancelled) if the user cancelled
//!         acc += i;
//!         ctx.advance(1);
//!     }
//!     Ok(acc)
//! });
//! assert_eq!(handle.join().unwrap(), 4950);
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;

/// Why a job did not produce a result.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// The user cancelled the job.
    #[error("cancelled")]
    Cancelled,
    /// The job failed.
    #[error("{0}")]
    Failed(String),
    /// The job panicked; this is a bug. Holds a description naming the job and the
    /// panic message.
    #[error("{0}")]
    Panicked(String),
}

#[derive(Debug, Default)]
struct Shared {
    cancelled: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
}

/// Handed to the job body: report progress and check for cancellation.
#[derive(Debug, Clone)]
pub struct JobContext {
    shared: Arc<Shared>,
}

impl JobContext {
    /// Set the total amount of work units (for the progress bar).
    pub fn set_total(&self, total: u64) {
        self.shared.total.store(total, Ordering::Relaxed);
    }

    /// Mark `units` more work as done.
    pub fn advance(&self, units: u64) {
        self.shared.done.fetch_add(units, Ordering::Relaxed);
    }

    /// Set the amount of work done so far, for callers that report absolute progress
    /// (for example `(done, total)` callbacks).
    pub fn set_done(&self, done: u64) {
        self.shared.done.store(done, Ordering::Relaxed);
    }

    /// True once cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.shared.cancelled.load(Ordering::Relaxed)
    }

    /// Return `Err(JobError::Cancelled)` if cancellation was requested. Call it often.
    pub fn checkpoint(&self) -> Result<(), JobError> {
        if self.is_cancelled() {
            Err(JobError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Where the job's thread stands, from the owner's side.
#[derive(Debug)]
enum State<T> {
    /// The thread is running or has finished but was not joined yet.
    Running(JoinHandle<Result<T, JobError>>),
    /// The thread could not be started; this is the error to report.
    Failed(JobError),
    /// The result was taken by [`JobHandle::join`].
    Joined,
}

/// Owner side of a running job.
///
/// Dropping the handle without joining it asks the job to stop at its next checkpoint,
/// so closed views and replaced jobs do not keep burning CPU in the background.
#[derive(Debug)]
pub struct JobHandle<T> {
    name: String,
    shared: Arc<Shared>,
    state: State<T>,
}

impl<T> JobHandle<T> {
    /// Job name, for the jobs panel.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Progress in `0.0..=1.0`, or `None` while the total is unknown.
    pub fn progress(&self) -> Option<f32> {
        let total = self.shared.total.load(Ordering::Relaxed);
        (total > 0).then(|| {
            let done = self.shared.done.load(Ordering::Relaxed).min(total);
            done as f32 / total as f32
        })
    }

    /// Ask the job to stop at its next checkpoint.
    pub fn cancel(&self) {
        self.shared.cancelled.store(true, Ordering::Relaxed);
    }

    /// True when the job body has returned (or the job could not be started).
    pub fn is_finished(&self) -> bool {
        match &self.state {
            State::Running(t) => t.is_finished(),
            State::Failed(_) | State::Joined => true,
        }
    }

    /// Wait for the result.
    pub fn join(mut self) -> Result<T, JobError> {
        match std::mem::replace(&mut self.state, State::Joined) {
            State::Running(t) => t.join().unwrap_or_else(|payload| {
                Err(JobError::Panicked(panic_message(
                    &self.name,
                    payload.as_ref(),
                )))
            }),
            State::Failed(e) => Err(e),
            State::Joined => Err(JobError::Failed(format!(
                "the result of job '{}' was already collected",
                self.name
            ))),
        }
    }
}

impl<T> Drop for JobHandle<T> {
    fn drop(&mut self) {
        if matches!(self.state, State::Running(_)) {
            self.cancel();
        }
    }
}

/// User-facing text for a job whose body panicked.
fn panic_message(name: &str, payload: &(dyn std::any::Any + Send)) -> String {
    let msg = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no panic message");
    format!("job '{name}' stopped unexpectedly (internal error: {msg}); please report this")
}

/// Run `body` on a background thread named after the job.
///
/// If the operating system cannot start a thread, the returned handle is already
/// finished and [`JobHandle::join`] reports the problem; this never panics.
pub fn spawn<T, F>(name: impl Into<String>, body: F) -> JobHandle<T>
where
    T: Send + 'static,
    F: FnOnce(&JobContext) -> Result<T, JobError> + Send + 'static,
{
    let name = name.into();
    let shared = Arc::new(Shared::default());
    let ctx = JobContext {
        shared: Arc::clone(&shared),
    };
    let started = std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || body(&ctx));
    handle_from(name, shared, started)
}

/// Build the handle for a thread that was (or could not be) started.
fn handle_from<T>(
    name: String,
    shared: Arc<Shared>,
    started: std::io::Result<JoinHandle<Result<T, JobError>>>,
) -> JobHandle<T> {
    let state = match started {
        Ok(t) => State::Running(t),
        Err(e) => State::Failed(JobError::Failed(format!(
            "could not start background job '{name}': {e}; close other jobs or tools and \
             try again"
        ))),
    };
    JobHandle {
        name,
        shared,
        state,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn reports_progress_and_result() {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let (half_tx, half_rx) = mpsc::channel::<()>();
        let h = spawn("count", move |ctx| {
            ctx.set_total(10);
            ctx.advance(5);
            half_tx.send(()).ok();
            go_rx.recv().ok();
            ctx.advance(5);
            Ok(42)
        });
        half_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!((h.progress().unwrap() - 0.5).abs() < 1e-6);
        go_tx.send(()).unwrap();
        assert_eq!(h.join().unwrap(), 42);
    }

    #[test]
    fn set_done_reports_absolute_progress() {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let (set_tx, set_rx) = mpsc::channel::<()>();
        let h = spawn("absolute", move |ctx| {
            ctx.set_total(8);
            ctx.set_done(6);
            ctx.set_done(2);
            set_tx.send(()).ok();
            go_rx.recv().ok();
            Ok(())
        });
        set_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!((h.progress().unwrap() - 0.25).abs() < 1e-6);
        go_tx.send(()).unwrap();
        h.join().unwrap();
    }

    #[test]
    fn cancellation_stops_the_job() {
        let h = spawn("forever", |ctx| {
            loop {
                ctx.checkpoint()?;
                std::thread::sleep(Duration::from_millis(1));
            }
            #[allow(unreachable_code)]
            Ok(())
        });
        h.cancel();
        assert!(matches!(h.join(), Err(JobError::Cancelled)));
    }

    #[test]
    fn panics_become_errors() {
        let h = spawn("boom", |_ctx| -> Result<(), JobError> { panic!("bug") });
        let Err(JobError::Panicked(msg)) = h.join() else {
            panic!("expected a panic error")
        };
        assert!(msg.contains("'boom'") && msg.contains("bug"), "{msg}");
        let h = spawn("boom2", |_ctx| -> Result<(), JobError> {
            panic!("value {}", 7)
        });
        let Err(JobError::Panicked(msg)) = h.join() else {
            panic!("expected a panic error")
        };
        assert!(msg.contains("value 7"), "{msg}");
    }

    #[test]
    fn dropping_the_handle_stops_the_job() {
        let (exit_tx, exit_rx) = mpsc::channel::<()>();
        let (started_tx, started_rx) = mpsc::channel::<()>();
        let h = spawn("forever", move |ctx| {
            started_tx.send(()).ok();
            let r = loop {
                if let Err(e) = ctx.checkpoint() {
                    break Err(e);
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            exit_tx.send(()).ok();
            r
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        drop::<JobHandle<()>>(h);
        exit_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn thread_start_failure_is_an_error() {
        let h: JobHandle<()> = handle_from(
            "big".to_owned(),
            Arc::new(Shared::default()),
            Err(std::io::Error::other("out of threads")),
        );
        assert!(h.is_finished());
        let Err(JobError::Failed(msg)) = h.join() else {
            panic!("expected a failure")
        };
        assert!(
            msg.contains("'big'") && msg.contains("out of threads"),
            "{msg}"
        );
    }

    #[test]
    fn threads_are_named_after_the_job() {
        let h = spawn("named", |_ctx| {
            Ok(std::thread::current().name().map(str::to_owned))
        });
        assert_eq!(h.join().unwrap().as_deref(), Some("named"));
    }
}
