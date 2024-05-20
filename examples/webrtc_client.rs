use buttplug::{client::{ButtplugClientEvent, ButtplugClient}, util::in_process_client, core::{connector::{ButtplugRemoteClientConnector}, message::serializer::ButtplugClientJSONSerializer}};
use buttplug_matchbox::ButtplugWebRtcTransport;
use futures::StreamExt;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tracing::Level;
use tracing_subscriber::{
  filter::{EnvFilter, LevelFilter},
  layer::SubscriberExt,
  util::SubscriberInitExt,
};

async fn wait_for_input() {
  BufReader::new(io::stdin())
    .lines()
    .next_line()
    .await
    .unwrap();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::registry()
  .with(tracing_subscriber::fmt::layer())
  //.with(sentry_tracing::layer())
  .with(
    EnvFilter::try_from_default_env()
      .or_else(|_| EnvFilter::try_new("info"))
      .unwrap(),
  )
  .try_init()
  .unwrap();
  // Usual embedded connector setup. We'll assume the server found all
  // of the subtype managers for us (the default features include all of them).
  //let client = in_process_client("Example Client", false).await;
  // To create a Websocket Connector, you need the websocket address and some generics fuckery.
  //let connector = new_json_ws_client_connector("ws://192.168.123.107:12345/buttplug");
  let connector = ButtplugRemoteClientConnector::<
      ButtplugWebRtcTransport,
      ButtplugClientJSONSerializer,
    >::new(ButtplugWebRtcTransport::new("ws://localhost:3536", ""));
  let client = ButtplugClient::new("Example Client");
  client.connect(connector).await?;
  let mut events = client.event_stream();

  // Set up our DeviceAdded/DeviceRemoved/ScanningFinished event handlers before connecting.
  tokio::spawn(async move {
    while let Some(event) = events.next().await {
      match event {
        ButtplugClientEvent::DeviceAdded(device) => {
          println!("Device {} Connected!", device.name());
        }
        ButtplugClientEvent::DeviceRemoved(info) => {
          println!("Device {} Removed!", info.name());
        }
        ButtplugClientEvent::ScanningFinished => {
          println!("Device scanning is finished!");
        }
        _ => {}
      }
    }
  });

  // We're connected, yay!
  println!("Connected!");

  // Now we can start scanning for devices, and any time a device is
  // found, we should see the device name printed out.
  client.start_scanning().await?;
  wait_for_input().await;

  // Some Subtype Managers will scan until we still them to stop, so
  // let's stop them now.
  client.stop_scanning().await?;
  wait_for_input().await;

  // Since we've scanned, the client holds information about devices it
  // knows about for us. These devices can be accessed with the Devices
  // getter on the client.
  println!("Client currently knows about these devices:");
  for device in client.devices() {
    println!("- {}", device.name());
  }
  wait_for_input().await;

  // And now we disconnect as usual.
  client.disconnect().await?;

  Ok(())
}