use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::child::bootstrap::ControlSender;
use crate::error::Result;
use pork_proto::protocol::{PorkChildStatus, PorkControlMessage, PorkStatusUpdate};

/// Automatic status reporter for child processes.
///
/// `StatusReporter` sends periodic heartbeat messages to the host process over the control channel.
/// This allows the host to observe lifecycle progress and cache the child's latest
/// reported status without mixing those updates into the application data plane.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
///
/// use pork::child::bootstrap::ChildBootstrap;
/// use pork::child::status_reporter::StatusReporter;
/// use pork_proto::protocol::PorkChildStatus;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let channels = ChildBootstrap::from_default_env()?.connect().await?;
/// // Reuse the control sender from the established child bootstrap connection.
/// let mut status_reporter = StatusReporter::new(
///     channels.control_sender(),
///     Duration::from_secs(5),
/// );
///
/// status_reporter.start().await?;
/// status_reporter.set_status(PorkChildStatus::Running).await;
/// status_reporter.set_status(PorkChildStatus::Stopping).await;
/// status_reporter.stop().await;
/// # Ok(())
/// # }
/// ```
pub struct StatusReporter {
    control_sender: ControlSender,
    interval: Duration,
    current_status: Arc<AsyncMutex<PorkChildStatus>>,
    task_handle: Option<JoinHandle<()>>,
}

impl StatusReporter {
    /// Creates a new status reporter.
    ///
    /// The status reporter will periodically send heartbeat messages to the host
    /// containing the current status and timestamp over the encoded control channel.
    ///
    /// # Arguments
    ///
    /// * `control_sender` - IPC channel sender for control messages
    /// * `interval` - Heartbeat interval (e.g., 5 seconds)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use pork::child::bootstrap::ChildBootstrap;
    /// use pork::child::status_reporter::StatusReporter;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let channels = ChildBootstrap::from_default_env()?.connect().await?;
    /// // The reporter starts in `Starting` and uses the negotiated control codec.
    /// let reporter = StatusReporter::new(channels.control_sender(), Duration::from_secs(5));
    /// let _ = reporter;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(control_sender: ControlSender, interval: Duration) -> Self {
        Self {
            control_sender,
            interval,
            current_status: Arc::new(AsyncMutex::new(PorkChildStatus::Starting)),
            task_handle: None,
        }
    }

    /// Updates the current status.
    ///
    /// This updates the status that will be sent in the next heartbeat.
    /// The status is shared across the background heartbeat task.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use pork::child::bootstrap::ChildBootstrap;
    /// use pork::child::status_reporter::StatusReporter;
    /// use pork_proto::protocol::PorkChildStatus;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let channels = ChildBootstrap::from_default_env()?.connect().await?;
    /// let reporter = StatusReporter::new(channels.control_sender(), Duration::from_secs(5));
    /// // Update the status that the next heartbeat will publish to the host.
    /// reporter.set_status(PorkChildStatus::Running).await;
    /// reporter.set_status(PorkChildStatus::Stopping).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_status(&self, status: PorkChildStatus) {
        let mut current = self.current_status.lock().await;
        *current = status;
    }

    /// Starts the background heartbeat task.
    ///
    /// This spawns a background task that periodically sends status updates to the host.
    /// The task continues until [`Self::stop`] is called, this reporter is dropped,
    /// or the control channel closes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use pork::child::bootstrap::ChildBootstrap;
    /// use pork::child::status_reporter::StatusReporter;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let channels = ChildBootstrap::from_default_env()?.connect().await?;
    /// let mut reporter = StatusReporter::new(channels.control_sender(), Duration::from_secs(5));
    /// // Spawn the periodic heartbeat worker before entering the main child loop.
    /// reporter.start().await?;
    /// reporter.stop().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        if self.task_handle.is_some() {
            return Ok(());
        }

        let control_sender = self.control_sender.clone();
        let interval = self.interval;
        let current_status = self.current_status.clone();

        let task_handle = tokio::spawn(async move {
            loop {
                // Wait for the next heartbeat interval
                tokio::time::sleep(interval).await;

                // Get current status
                let status = {
                    let status_guard = current_status.lock().await;
                    *status_guard
                };

                // Create and send status update
                let update = PorkStatusUpdate {
                    status,
                    timestamp_ms: current_time_ms(),
                };
                let message = PorkControlMessage::StatusUpdate(update);

                if control_sender.send(message).is_err() {
                    break;
                }
            }
        });

        self.task_handle = Some(task_handle);
        Ok(())
    }

    /// Waits for the background heartbeat task to finish.
    ///
    /// This is useful after the task has already been stopped by the remote end or
    /// when another part of the program is responsible for shutting it down.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use pork::child::bootstrap::ChildBootstrap;
    /// use pork::child::status_reporter::StatusReporter;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let channels = ChildBootstrap::from_default_env()?.connect().await?;
    /// let mut reporter = StatusReporter::new(channels.control_sender(), Duration::from_secs(5));
    /// reporter.start().await?;
    /// // Once stopped, `wait_for_completion` just joins the already-cancelled task.
    /// reporter.stop().await;
    /// reporter.wait_for_completion().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }

    /// Stops the heartbeat worker and waits for cancellation to complete.
    ///
    /// Call this during child shutdown when you want explicit ownership over when
    /// periodic status reporting ends.
    pub async fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
    }
}

impl Drop for StatusReporter {
    fn drop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// Returns the current system time in milliseconds since Unix epoch.
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_ms_returns_positive_value() {
        let t = current_time_ms();
        assert!(
            t > 0,
            "timestamp should be a positive number of milliseconds"
        );
    }

    #[test]
    fn current_time_ms_is_monotonic() {
        let t1 = current_time_ms();
        std::thread::sleep(Duration::from_millis(1));
        let t2 = current_time_ms();
        assert!(t2 >= t1, "timestamp should be monotonic or equal");
    }
}
