use std::time::Duration;

use buttplug::{server::device::hardware::communication::{
  HardwareCommunicationManager, HardwareCommunicationManagerBuilder, HardwareCommunicationManagerEvent,
}, core::{connector::{transport::ButtplugTransportIncomingMessage, ButtplugRemoteClientConnector}, message::serializer::{ButtplugSerializedMessage, ButtplugClientJSONSerializer}}, client::ButtplugClient};
use futures::{FutureExt, select, StreamExt};
use futures_timer::Delay;
use matchbox_socket::{WebRtcSocket, PeerId, PeerState};
use tokio::sync::{mpsc::Sender, Notify};
use tracing::event;

use crate::{ButtplugWebRtcTransport, webrtc_device::WebRtcHardwareConnector};

pub struct WebRtcDeviceCommunicationManagerBuilder {
  server_name: String,
  signalling_server_address: String
}

impl WebRtcDeviceCommunicationManagerBuilder {
  pub fn new(signalling_server_address: &str, server_name: &str) -> Self {
    Self {
      server_name: server_name.to_owned(),
      signalling_server_address: signalling_server_address.to_owned()
    }
  }
}

impl HardwareCommunicationManagerBuilder for WebRtcDeviceCommunicationManagerBuilder {
    fn finish(
        &mut self,
        sender: tokio::sync::mpsc::Sender<buttplug::server::device::hardware::communication::HardwareCommunicationManagerEvent>,
      ) -> Box<dyn HardwareCommunicationManager> {
        Box::new(WebRtcDeviceCommunicationManager::new(sender, &self.server_name, &self.signalling_server_address))
    }
}

pub struct WebRtcDeviceCommunicationManager {
  event_stream: Sender<HardwareCommunicationManagerEvent>,
}

/*
async fn run_webrtc_client_socket(
  event_stream: Sender<HardwareCommunicationManagerEvent>,
  socket: WebRtcSocket,
  loop_fut: _
) {

}
*/

async fn run_webrtc_comm_manager(
  event_stream: Sender<HardwareCommunicationManagerEvent>,
  server_name: &str,
  signalling_server_address: &str
) {
  let connector = ButtplugRemoteClientConnector::<
      ButtplugWebRtcTransport,
      ButtplugClientJSONSerializer,
    >::new(ButtplugWebRtcTransport::new(signalling_server_address, ""));
  let client = ButtplugClient::new("Example Client");
  // Wait for someone to connect their server
  client.connect(connector).await;  
  let mut client_event_stream = client.event_stream();
  client.start_scanning().await;
  loop {
    select! {
      maybe_event = client_event_stream.next().fuse() => {
        if let Some(event) = maybe_event {
          match event {
            buttplug::client::ButtplugClientEvent::ScanningFinished => {
              info!("Scanning finished");
            },
            buttplug::client::ButtplugClientEvent::DeviceAdded(device) => {
              info!("Device added to webrtc device server: {:?}", device);
              event_stream.send(HardwareCommunicationManagerEvent::DeviceFound { name: device.name().clone(), address: format!("webrtc-{}", device.index()), creator: Box::new(WebRtcHardwareConnector::new(&device)) }).await;
            },
            buttplug::client::ButtplugClientEvent::DeviceRemoved(device) => {
              info!("Device removed from webrtc device server: {:?}", device);
            },
            buttplug::client::ButtplugClientEvent::PingTimeout => {
              info!("Got ping timeout from webrtc device server");
            }
            buttplug::client::ButtplugClientEvent::ServerConnect => {
              info!("Connected to webrtc device server");
            },
            buttplug::client::ButtplugClientEvent::ServerDisconnect => {
              info!("Disconnected from webrtc device server");
            },
            buttplug::client::ButtplugClientEvent::Error(_) => todo!(),
}
        } else {

        }
      }
    }
  }
}

impl WebRtcDeviceCommunicationManager {
  pub fn new(
    event_stream: Sender<HardwareCommunicationManagerEvent>,
    server_name: &str,
    signalling_server_address: &str
  ) -> Self {
    let event_stream_clone = event_stream.clone();
    let server_name = server_name.to_owned();
    let signalling_server_address = signalling_server_address.to_owned();
    tokio::spawn(async move {
      run_webrtc_comm_manager(
        event_stream_clone,
        &server_name,
        &signalling_server_address
      ).await;
    });
    Self {
      event_stream
    }
  }
}

impl HardwareCommunicationManager for WebRtcDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "WebRTC Communication Manager"
  }

  fn start_scanning(&mut self) -> buttplug::core::ButtplugResultFuture {
    async move {
      Ok(())
    }.boxed()
  }

  fn stop_scanning(&mut self) -> buttplug::core::ButtplugResultFuture {
    async move {
      Ok(())
    }.boxed()
  }

  fn can_scan(&self) -> bool {
    true
  }
}
