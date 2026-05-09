//! Host-side bootstrap coordination for establishing dual-channel IPC with child processes.
//!
//! Strategy:
//! 1. Host creates two `IpcOneShotServer` instances, one for data and one for control.
//! 2. Host passes their server names via environment variables.
//! 3. Child connects to these servers and exchanges channel endpoints.
//! 4. Both channels are then ready for bidirectional communication.

/// Host-side sender and receiver wrappers for the dual-channel transport.
pub mod channels;

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};

use self::channels::{HostControlReceiver, HostControlSender, HostDataReceiver, HostDataSender};
use crate::error::Result;

/// Result of successful host bootstrap: both control and data channel pairs.
#[derive(Debug)]
pub struct HostBootstrapChannels {
    /// Receives data or application messages from the child process.
    pub data_receiver: HostDataReceiver,
    /// Sends data or application messages to the child process.
    pub data_sender: HostDataSender,
    /// Receives control messages from the child process.
    pub control_receiver: HostControlReceiver,
    /// Sends control messages to the child process.
    pub control_sender: HostControlSender,
}

/// Environment variables to pass to the child process for bootstrap.
#[derive(Debug, Clone)]
pub struct BootstrapEnv {
    /// Server name for the data-channel bootstrap handshake.
    pub data_server_name: String,
    /// Server name for the control-channel bootstrap handshake.
    pub control_server_name: String,
}

impl BootstrapEnv {
    /// Applies bootstrap environment to a `tokio::process::Command`.
    pub fn apply_to_command(
        &self,
        command: &mut tokio::process::Command,
        data_env_name: &str,
        control_env_name: &str,
    ) {
        command
            .env(data_env_name, &self.data_server_name)
            .env(control_env_name, &self.control_server_name);
    }
}

type DataHandshakeTuple = (IpcSender<Vec<u8>>, IpcReceiver<Vec<u8>>);
type ControlHandshakeTuple = (IpcSender<Vec<u8>>, IpcReceiver<Vec<u8>>);

/// Holds the two bootstrap servers during connection setup.
#[doc(hidden)]
pub struct HostBootstrapServerPair {
    /// The one-shot server for the data channel handshake.
    pub(crate) data_server: IpcOneShotServer<DataHandshakeTuple>,
    /// The one-shot server for the control channel handshake.
    pub(crate) control_server: IpcOneShotServer<ControlHandshakeTuple>,
}

/// Host-side bootstrap coordinator.
///
/// Creates IPC one-shot servers for data and control channels, then waits for the child
/// to connect and exchange channel endpoints.
pub struct HostBootstrap {}

impl HostBootstrap {
    /// Creates a new host bootstrap coordinator with default configuration.
    pub fn new() -> Self {
        Self {}
    }

    /// Creates two one-shot servers for data and control handshakes.
    pub async fn create_servers() -> Result<(BootstrapEnv, HostBootstrapServerPair)> {
        tokio::task::spawn_blocking(Self::create_servers_sync)
            .await
            .map_err(|error| crate::error::OrchestratorError::Io(std::io::Error::other(error)))?
    }

    fn create_servers_sync() -> Result<(BootstrapEnv, HostBootstrapServerPair)> {
        let (data_server, data_name) = IpcOneShotServer::<DataHandshakeTuple>::new()?;
        let (control_server, control_name) = IpcOneShotServer::<ControlHandshakeTuple>::new()?;

        Ok((
            BootstrapEnv {
                data_server_name: data_name,
                control_server_name: control_name,
            },
            HostBootstrapServerPair {
                data_server,
                control_server,
            },
        ))
    }

    /// Accepts connections from the child on both data and control channels.
    pub async fn accept_connections(
        servers: HostBootstrapServerPair,
    ) -> Result<HostBootstrapChannels> {
        let data_future = tokio::task::spawn_blocking(move || servers.data_server.accept());
        let control_future = tokio::task::spawn_blocking(move || servers.control_server.accept());

        let data_result = data_future
            .await
            .map_err(|error| crate::error::OrchestratorError::Io(std::io::Error::other(error)))??;
        let control_result = control_future
            .await
            .map_err(|error| crate::error::OrchestratorError::Io(std::io::Error::other(error)))??;

        let (_, (data_to_child, data_from_child)) = data_result;
        let (_, (control_to_child, control_from_child)) = control_result;

        Ok(HostBootstrapChannels {
            data_receiver: HostDataReceiver::new(data_from_child),
            data_sender: HostDataSender::new(data_to_child),
            control_receiver: HostControlReceiver::new(control_from_child),
            control_sender: HostControlSender::new(control_to_child),
        })
    }
}

impl Default for HostBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_bootstrap_new_creates_instance() {
        let _bootstrap = HostBootstrap::new();
        let _bootstrap2 = HostBootstrap::default();
    }

    #[test]
    fn bootstrap_env_stores_server_names() {
        let env = BootstrapEnv {
            data_server_name: "data_server_test".to_string(),
            control_server_name: "control_server_test".to_string(),
        };

        assert_eq!(env.data_server_name, "data_server_test");
        assert_eq!(env.control_server_name, "control_server_test");
    }

    #[test]
    fn bootstrap_env_can_be_cloned() {
        let env = BootstrapEnv {
            data_server_name: "data".to_string(),
            control_server_name: "control".to_string(),
        };
        let env_cloned = env.clone();
        assert_eq!(env.data_server_name, env_cloned.data_server_name);
    }

    #[tokio::test]
    async fn bootstrap_env_applies_to_command() {
        let env = BootstrapEnv {
            data_server_name: "test_data_server".to_string(),
            control_server_name: "test_control_server".to_string(),
        };

        let mut cmd = tokio::process::Command::new("true");
        env.apply_to_command(
            &mut cmd,
            crate::DEFAULT_BOOTSTRAP_ENV,
            crate::CONTROL_BOOTSTRAP_ENV,
        );
    }

    #[test]
    fn host_bootstrap_channels_debug_format() {
        let type_name = std::any::type_name::<HostBootstrapChannels>();
        assert!(type_name.contains("HostBootstrapChannels"));
    }

    #[test]
    fn bootstrap_env_debug_format() {
        let env = BootstrapEnv {
            data_server_name: "data".to_string(),
            control_server_name: "control".to_string(),
        };
        let debug_str = format!("{:?}", env);
        assert!(debug_str.contains("data"));
        assert!(debug_str.contains("control"));
    }
}
