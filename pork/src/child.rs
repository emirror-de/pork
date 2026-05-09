/// Child-side bootstrap helpers and shared child process setup.
///
/// This module contains the functions and types a managed child process uses to:
/// - read bootstrap information from the environment,
/// - connect back to the parent over IPC for both data and control traffic,
/// - report lifecycle status over the encoded control channel.
pub mod bootstrap;
/// Automatic status reporter for sending periodic heartbeat updates to the parent.
pub mod status_reporter;
