use std::time::Duration;

use pork::child::bootstrap::ChildBootstrap;
use pork::child::status_reporter::StatusReporter;
use pork_comms::{ChildMessage, HostMessage, decode_message, encode_message};
use pork_proto::protocol::{PorkChildStatus, PorkControlCodec, PorkControlMessage};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channels = ChildBootstrap::from_default_env()?.connect().await?;
    let control_codec = channels.control_codec();
    let mut handled_messages = 0_usize;
    let mut status_reporter = StatusReporter::new(channels.control_sender(), HEARTBEAT_INTERVAL);

    status_reporter.start().await?;
    status_reporter.set_status(PorkChildStatus::Running).await;

    println!(
        "child: connected to host with control codec {} and {}s heartbeats",
        control_codec.as_env_value(),
        HEARTBEAT_INTERVAL.as_secs()
    );

    channels.send_data(encode_message(
        control_codec,
        ChildMessage::Ready {
            codec: control_codec_name(control_codec).to_owned(),
        },
    )?)?;

    loop {
        tokio::select! {
            control = channels.recv_control() => {
                match control? {
                    Some(PorkControlMessage::GracefulShutdown | PorkControlMessage::Restart) => {
                        status_reporter.set_status(PorkChildStatus::Stopping).await;
                        break;
                    }
                    Some(PorkControlMessage::StatusUpdate(_)) => {}
                    None => break,
                }
            }
            payload = channels.recv_data() => {
                let Some(payload) = payload else {
                    println!("child: data channel closed");
                    break;
                };

                let Some(message) = decode_message::<HostMessage>(control_codec, payload.as_ref())? else {
                    continue;
                };

                handled_messages += 1;

                match message {
                    HostMessage::Echo(text) => {
                        println!("child: received echo request '{text}'");
                        channels.send_data(encode_message(control_codec, ChildMessage::Echoed(text))?)?;
                    }
                    HostMessage::Status => {
                        println!("child: received status request");
                        channels.send_data(encode_message(
                            control_codec,
                            ChildMessage::Status {
                                pid: std::process::id(),
                                handled_messages,
                                codec: control_codec_name(control_codec).to_owned(),
                            },
                        )?)?;
                    }
                }
            }
        }
    }

    status_reporter.stop().await;
    println!("child: exiting cleanly");
    Ok(())
}

fn control_codec_name(codec: PorkControlCodec) -> &'static str {
    match codec {
        PorkControlCodec::Json => "json",
        PorkControlCodec::Postcard => "postcard",
    }
}
