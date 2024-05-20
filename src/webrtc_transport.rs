use std::{sync::Arc, time::Duration};

use buttplug::{
  core::{
    connector::{
      transport::{ButtplugConnectorTransport, ButtplugTransportIncomingMessage},
      ButtplugConnectorError, ButtplugConnectorResultFuture,
    },
    message::serializer::ButtplugSerializedMessage,
  },
  util::async_manager,
};
use futures::{future::BoxFuture, select, FutureExt};
use futures_timer::Delay;
use matchbox_socket::{PeerId, PeerState, WebRtcSocket};
use tokio::sync::{
  mpsc::{Receiver, Sender},
  Notify,
};

pub struct ButtplugWebRtcTransport {
  room_name: String,
  signalling_server_address: String,
  /// Internally held sender, used for when disconnect is called.
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugWebRtcTransport {
  pub fn new(signalling_server_address: &str, room_name: &str) -> Self {
    Self {
      room_name: room_name.to_owned(),
      signalling_server_address: signalling_server_address.to_owned(),
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

impl ButtplugConnectorTransport for ButtplugWebRtcTransport {
  fn connect(
    &self,
    mut outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let room_name = self.signalling_server_address.clone();

    async move {
      let (mut socket, loop_fut) = WebRtcSocket::new_reliable(room_name); //"ws://localhost:3536/");
      let peer_notifier = Arc::new(Notify::new());
      let notifier_clone = peer_notifier.clone();
      async_manager::spawn(async move {
        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);

        let timeout = Delay::new(Duration::from_millis(10));
        futures::pin_mut!(timeout);

        let mut client_peer: Option<PeerId> = None;

        loop {
          // Handle any new peers
          for (peer, state) in socket.update_peers() {
            match state {
              PeerState::Connected => {
                peer_notifier.notify_waiters();
                client_peer = Some(peer);
                info!("Peer joined: {peer}");
              }
              PeerState::Disconnected => {
                info!("Peer left: {peer}");
              }
            }
          }

          // Accept any messages incoming
          for (peer, packet) in socket.receive() {
            let message = String::from_utf8_lossy(&packet);
            let _ = incoming_sender
              .send(ButtplugTransportIncomingMessage::Message(
                ButtplugSerializedMessage::Text(message.to_string()),
              ))
              .await;
            info!("Message from {peer}: {message:?}");
          }

          if let Some(client) = client_peer {
            select! {
                // Restart this loop every 100ms
                _ = (&mut timeout).fuse() => {
                  timeout.reset(Duration::from_millis(10));
                }

                // Or break if the message loop ends (disconnected, closed, etc.)
                _ = &mut loop_fut => {
                  break;
                }

                msg = outgoing_receiver.recv().fuse() => {
                  match msg {
                    Some(ButtplugSerializedMessage::Text(txt)) => {
                      let packet = txt.as_bytes().to_vec().into_boxed_slice();
                      socket.send(packet, client_peer.as_ref().unwrap().clone());
                    }
                    Some(ButtplugSerializedMessage::Binary(bin)) => {
                      socket.send(bin.into(), client_peer.as_ref().unwrap().clone());
                    }
                    None => {
                      break;
                    }
                  }
                }
            }
          } else {
            select! {
              // Restart this loop every 100ms
              _ = (&mut timeout).fuse() => {
                timeout.reset(Duration::from_millis(10));
              }

              // Or break if the message loop ends (disconnected, closed, etc.)
              _ = &mut loop_fut => {
                break;
              }
            }
          }
        }
      });
      info!("waiting for peer");
      notifier_clone.notified().await;
      info!("got peer");
      Ok(())
    }
    .boxed()
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    async move {
      // If we can't send the message, we have no loop, so we're not connected.
      //disconnect_notifier.notify_waiters();
      Ok(())
    }
    .boxed()
  }
}
