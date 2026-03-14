use pork::DEFAULT_BOOTSTRAP_ENV;
use pork::child::{child_connect_from_env, child_control_codec_from_env};
use pork_comms::{ChildMessage, HostMessage, decode_message, encode_message};
use pork_proto::protocol::PorkControlCodec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let control_codec = child_control_codec_from_env()?;
    let (from_host, to_host) = child_connect_from_env(DEFAULT_BOOTSTRAP_ENV)?;
    let mut handled_messages = 0_usize;

    println!(
        "child: connected to host with control codec {}",
        control_codec.as_env_value()
    );

    to_host.send(encode_message(
        control_codec,
        ChildMessage::Ready {
            codec: control_codec_name(control_codec).to_owned(),
        },
    )?)?;

    loop {
        let payload = from_host.recv()?;

        if control_codec.is_graceful_shutdown_message(&payload) {
            println!("child: received graceful shutdown request");
            break;
        }

        let Some(message) = decode_message::<HostMessage>(control_codec, &payload)? else {
            continue;
        };

        handled_messages += 1;

        match message {
            HostMessage::Echo(text) => {
                println!("child: received echo request '{text}'");
                to_host.send(encode_message(control_codec, ChildMessage::Echoed(text))?)?;
            }
            HostMessage::Status => {
                println!("child: received status request");
                to_host.send(encode_message(
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

    println!("child: exiting cleanly");
    Ok(())
}

fn control_codec_name(codec: PorkControlCodec) -> &'static str {
    match codec {
        PorkControlCodec::Json => "json",
        PorkControlCodec::Postcard => "postcard",
    }
}
