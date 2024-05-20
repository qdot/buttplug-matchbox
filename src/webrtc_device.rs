use core::fmt;
use std::{sync::Arc, fmt::Debug};

use buttplug::{server::device::{hardware::{HardwareInternal, HardwareConnector, HardwareSpecializer, Hardware, GenericHardwareSpecializer, HardwareEvent}, configuration::{ProtocolCommunicationSpecifier, WebsocketSpecifier}}, client::ButtplugClientDevice, core::{errors::ButtplugDeviceError, message::{Endpoint, ButtplugDeviceCommandMessageUnion, ActuatorType}}};
use futures::FutureExt;
use async_trait::async_trait;
use tokio::sync::broadcast::Sender;

pub struct WebRtcHardwareConnector {
  device: Arc<ButtplugClientDevice>
}

impl WebRtcHardwareConnector {
  pub fn new(device: &Arc<ButtplugClientDevice>) -> Self {
    Self { device: device.clone() }
  }
}

impl Debug for WebRtcHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("WebRtcHardwareConnector")
      .field("name", &self.device.name())
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for WebRtcHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::Websocket(WebsocketSpecifier::new(&vec!["webrtc".to_owned()]))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    debug!("Emitting a new xbox device impl.");
    let hardware_internal = WebRtcHardware::new(&self.device);
    let hardware = Hardware::new(
      &self.device.name(),
      &format!("webrtc-device-{}", self.device.index()).to_owned(),
      &[Endpoint::Tx, Endpoint::Rx],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

pub struct WebRtcHardware {
  device: Arc<ButtplugClientDevice>,
  event_stream: Sender<HardwareEvent>
}

impl WebRtcHardware {
  pub fn new(device: &Arc<ButtplugClientDevice>) -> Self {
    let (sender, _) = tokio::sync::broadcast::channel(255);
    Self {
      device: device.clone(),
      event_stream: sender
    }
  }
}

impl HardwareInternal for WebRtcHardware {
  fn disconnect(
    &self,
  ) -> futures::future::BoxFuture<'static, Result<(), buttplug::core::errors::ButtplugDeviceError>>
  {
    info!("Disconnect called on remote device, noop");
    async move {
      Ok(())
    }.boxed()
  }

  fn event_stream(
    &self,
  ) -> tokio::sync::broadcast::Receiver<buttplug::server::device::hardware::HardwareEvent> {
    self.event_stream.subscribe()
  }

  fn read_value(
    &self,
    msg: &buttplug::server::device::hardware::HardwareReadCmd,
  ) -> futures::future::BoxFuture<
    'static,
    Result<
      buttplug::server::device::hardware::HardwareReading,
      buttplug::core::errors::ButtplugDeviceError,
    >,
  > {
    todo!()
  }

  fn write_value(
    &self,
    msg: &buttplug::server::device::hardware::HardwareWriteCmd,
  ) -> futures::future::BoxFuture<'static, Result<(), buttplug::core::errors::ButtplugDeviceError>>
  {
    // WebRTC sockets will always be wrapped as passthru. We went to get the message here so we can
    // reformat it and let the client send it on.
    let msg_str = String::from_utf8(msg.data().clone()).unwrap();
    let scalar_cmd: ButtplugDeviceCommandMessageUnion = serde_json::from_str(&msg_str).unwrap();
    let mut scalar_amount: Option<f64> = None;
    if let ButtplugDeviceCommandMessageUnion::ScalarCmd(cmd) = scalar_cmd {
      scalar_amount = Some(cmd.scalars()[0].scalar());
    } else {
      warn!("Got non-forwardable message: {}", msg_str);
    }
    let device = self.device.clone();
    async move {
      if let Some(amount) = scalar_amount {
        device.scalar(&buttplug::client::ScalarCommand::Scalar((amount, ActuatorType::Vibrate))).await;
      }
      Ok(())
    }.boxed()
  }

  fn subscribe(
    &self,
    msg: &buttplug::server::device::hardware::HardwareSubscribeCmd,
  ) -> futures::future::BoxFuture<'static, Result<(), buttplug::core::errors::ButtplugDeviceError>>
  {
    todo!()
  }

  fn unsubscribe(
    &self,
    msg: &buttplug::server::device::hardware::HardwareUnsubscribeCmd,
  ) -> futures::future::BoxFuture<'static, Result<(), buttplug::core::errors::ButtplugDeviceError>>
  {
    todo!()
  }
}
