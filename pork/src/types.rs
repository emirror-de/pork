use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Stable identifier assigned to a managed child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Creates a process identifier from a raw numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u64> for ProcessId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ProcessId> for u64 {
    fn from(value: ProcessId) -> Self {
        value.get()
    }
}

/// Managed name used to identify a child process inside an orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ManagedChildName(String);

impl ManagedChildName {
    /// Creates a managed child name from an owned string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the managed name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ManagedChildName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ManagedChildName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for ManagedChildName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ManagedChildName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Raw application payload sent over the data channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPayload(Vec<u8>);

impl DataPayload {
    /// Creates a data payload from owned bytes.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Returns the payload bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Converts the payload into its owned bytes.
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for DataPayload {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<DataPayload> for Vec<u8> {
    fn from(value: DataPayload) -> Self {
        value.into_inner()
    }
}

impl From<String> for DataPayload {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<&str> for DataPayload {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl From<&Vec<u8>> for DataPayload {
    fn from(value: &Vec<u8>) -> Self {
        Self(value.clone())
    }
}

impl From<Vec<u8>> for DataPayload {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for DataPayload {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for DataPayload {
    fn from(value: [u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for DataPayload {
    fn from(value: &[u8; N]) -> Self {
        Self(value.to_vec())
    }
}

/// Raw encoded framework payload sent over the control channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPayload(Vec<u8>);

impl ControlPayload {
    /// Creates a control payload from owned bytes.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Returns the payload bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Converts the payload into its owned bytes.
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for ControlPayload {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<ControlPayload> for Vec<u8> {
    fn from(value: ControlPayload) -> Self {
        value.into_inner()
    }
}

impl From<String> for ControlPayload {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<&str> for ControlPayload {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl From<&Vec<u8>> for ControlPayload {
    fn from(value: &Vec<u8>) -> Self {
        Self(value.clone())
    }
}

impl From<Vec<u8>> for ControlPayload {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for ControlPayload {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for ControlPayload {
    fn from(value: [u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for ControlPayload {
    fn from(value: &[u8; N]) -> Self {
        Self(value.to_vec())
    }
}

/// Environment variable name used for a bootstrap handshake.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BootstrapEnvName(String);

impl BootstrapEnvName {
    /// Creates a bootstrap environment-variable name from an owned string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the environment-variable name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BootstrapEnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for BootstrapEnvName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for BootstrapEnvName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for BootstrapEnvName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Child executable path used by a [`crate::spec::ProcessSpec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessExecutable(PathBuf);

impl ProcessExecutable {
    /// Creates an executable path from an owned path value.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the executable path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for ProcessExecutable {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<PathBuf> for ProcessExecutable {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<&Path> for ProcessExecutable {
    fn from(value: &Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl From<&str> for ProcessExecutable {
    fn from(value: &str) -> Self {
        Self(PathBuf::from(value))
    }
}

/// Log-file path used for child stdout and stderr capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFilePath(PathBuf);

impl LogFilePath {
    /// Creates a log-file path from an owned path value.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the log-file path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for LogFilePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<PathBuf> for LogFilePath {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<&Path> for LogFilePath {
    fn from(value: &Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl From<&str> for LogFilePath {
    fn from(value: &str) -> Self {
        Self(PathBuf::from(value))
    }
}

/// Typed heartbeat interval used for child status reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeartbeatInterval(Duration);

impl HeartbeatInterval {
    /// Creates a heartbeat interval from a duration.
    pub const fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Returns the underlying duration.
    pub const fn as_duration(self) -> Duration {
        self.0
    }
}

impl From<Duration> for HeartbeatInterval {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<HeartbeatInterval> for Duration {
    fn from(value: HeartbeatInterval) -> Self {
        value.as_duration()
    }
}
