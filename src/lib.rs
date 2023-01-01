//! A pure rust MQTT client which strives to be as efficient as possible.
//! This crate strives to provide an ergonomic API and design that fits Rust.
//! 
//! There are three parts to the design of the MQTT client. The network, the event handler and the client.
//! 
//! The network - which simply reads and forms packets from the network.
//! The event handler - which makes sure that the MQTT protocol is followed.
//! By providing a custom handler during the internal handling, messages are handled before they are acked.
//! The client - which is used to send messages from different places.
//! 
//! A few questions still remain:
//! - This crate uses async channels to perform communication across its parts. Is there a better approach?
//! - This crate provides network implementation which hinder sync and async agnosticity.
//! 
//! For the future it could be nice to be sync, async and runtime agnostic.
//! This can be achieved by decoupling the MQTT internals from the network communication.
//! The user could provide the received packets while this crate returns the response packets.
//! Another aproach could be providing the read bytes, however, QUIC supports multiple streams.
//!
//! Currently, we do provide network implementations for the smol and tokio runtimes that you can enable with feature flags.
//!
//! Tokio example:
//! ----------------------------
//! ```no_run
//! let config = RustlsConfig::Simple {
//!     ca: EMQX_CERT.to_vec(),
//!     alpn: None,
//!     client_auth: None,
//! };
//! let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string());
//! let (mqtt_network, handler, client) = create_tokio_rustls(opt, config);
//! 
//! task::spawn(async move {
//!     join!(mqtt_network.run(), hadnler.handle(/* Custom handler */));
//! });
//! 
//! for i in 0..10 {
//!     client.publish("test", QoS::AtLeastOnce, false, b"test payload").await.unwrap();
//!     time::sleep(Duration::from_millis(100)).await;
//! }
//! 
//! ```
//! 


#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;

#[cfg(all(feature = "smol", feature = "smol-rustls"))]
use connections::async_rustls::{TlsReader, TlsWriter};
#[cfg(all(feature = "tokio", feature = "tcp"))]
use connections::tcp::{TcpReader, TcpWriter};

use connections::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite};

use connections::transport::RustlsConfig;

use event_handler::EventHandlerTask;
use network::MqttNetwork;

mod available_packet_ids;
pub mod client;
pub mod connect_options;
pub mod connections;
pub mod error;
pub mod event_handler;
mod network;
pub mod packets;
mod state;
mod util;

#[cfg(test)]
mod tests;

#[cfg(all(feature = "smol", feature = "smol-rustls"))]
pub fn create_smol_rustls(
    mut options: ConnectOptions,
    tls_config: RustlsConfig,
) -> (
    MqttNetwork<connections::async_rustls::TlsReader, connections::async_rustls::TlsWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    use connections::transport::TlsConfig;

    options.tls_config = Some(TlsConfig::Rustls(tls_config));
    new(options)
}

#[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
pub fn create_tokio_rustls(
    mut options: ConnectOptions,
    tls_config: RustlsConfig,
) -> (
    MqttNetwork<connections::tokio_rustls::TlsReader, connections::tokio_rustls::TlsWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    use connections::transport::TlsConfig;

    options.tls_config = Some(TlsConfig::Rustls(tls_config));
    new(options)
}

#[cfg(all(feature = "tokio", feature = "tcp"))]
pub fn create_tokio_tcp(
    options: ConnectOptions,
) -> (
    MqttNetwork<TcpReader, TcpWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    new(options)
}

pub fn new<R, W>(options: ConnectOptions) -> (MqttNetwork<R, W>, EventHandlerTask, AsyncClient)
where
    R: AsyncMqttNetworkRead<W = W>,
    W: AsyncMqttNetworkWrite,
{
    let receive_maximum = options.receive_maximum();

    let (to_network_s, to_network_r) = async_channel::bounded(100);
    let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
    let (client_to_handler_s, client_to_handler_r) =
        async_channel::bounded(receive_maximum as usize);

    let last_network_action = Arc::new(Mutex::new(Instant::now()));

    let (handler, packet_ids) = EventHandlerTask::new(
        &options,
        network_to_handler_r,
        to_network_s.clone(),
        client_to_handler_r.clone(),
        last_network_action.clone(),
    );

    let network = MqttNetwork::<R, W>::new(
        options,
        network_to_handler_s,
        to_network_r,
        last_network_action,
    );

    let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

    (network, handler, client)
}
