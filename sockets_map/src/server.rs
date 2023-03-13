use self::client::Client;
use super::host;
use crate::server::message::Message;
use anyhow::Result;
use log;
use std::{
    collections::HashMap,
    marker::{Send, Sync},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::select;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tsyncp::{self, broadcast::BincodeSender};

pub const DEFAULT_PORT: u16 = 6840;

pub mod client;
pub mod message;

/// The server will listen for clients unil `run` is set to `false`
pub async fn listen<FnSocket, FnClient1, FnClient2, FnClient3>(
    server_addr: String,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    run_token: CancellationToken,
    on_connect_callback: FnSocket,
    on_client_registration_callback: FnClient1,
    on_client_update_callback: FnClient2,
    on_client_exit_callback: FnClient3,
) -> Result<BincodeSender<Message>>
where
    FnSocket: Fn(SocketAddr) + Send + Sync + 'static,
    FnClient1: Fn(&Client) + Send + 'static,
    FnClient2: Fn(&Client) + Send + 'static,
    FnClient3: Fn(&Client) + Send + 'static,
{
    // Create channel
    let channel: tsyncp::multi_channel::BincodeChannel<Message> =
        tsyncp::multi_channel::channel_on(server_addr)
            .set_tcp_reuseaddr(true)
            .await?;
    let (mut rx, tx) = channel.split();

    tokio::spawn(async move {
        // Wait for clients to connect
        loop {
            // Wait a bit not to consume too much CPU
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Break out of the loop if the token is cancelled, instead of waiting for new connections
            let res = select! {
                _ = run_token.cancelled() => {
                    break;
                },
                (res, _) = rx.recv().with_addr().accepting().handle(&on_connect_callback) => {
                    res
                },
            };

            // Listen to incoming agents
            if let Some(res) = res {
                let (message, client_addr) = match res {
                    Ok((message, client_addr)) => (message, client_addr),
                    Err(e) => {
                        if e.is_connection_error() {
                            log::error!("{} disconnected", e.peer_addr().unwrap());
                        } else if e.is_decode_error() {
                            log::error!("decode error from {:?} ", e.peer_addr());
                        } else {
                            log::error!("other error from {:?} ", e.peer_addr());
                        }
                        continue;
                    }
                };
                log::debug!("received message: {message:#?}");

                let mut clients_mut = clients.write().await;
                log::debug!("clients: {clients_mut:#?}");
                match message {
                    Message::Register(r) => {
                        let client = Client::new(
                            r.hostname().to_owned(),
                            r.pretty_name().map(|r| r.to_string()),
                            r.ip_addresses().to_vec(),
                        );
                        on_client_registration_callback(&client);
                        clients_mut.insert(client_addr.to_string(), client);
                    }
                    Message::Update(update) => {
                        if let Some(client) = clients_mut.get_mut(&client_addr.to_string()) {
                            client.add_update(update);
                            on_client_update_callback(client);
                        } else {
                            log::error!("unknown client: {}", client_addr);
                        }
                    }
                    Message::Exit => {
                        if let Some(client) = clients_mut.get_mut(&client_addr.to_string()) {
                            on_client_exit_callback(client);
                            clients_mut.remove(&client_addr.to_string());
                        } else {
                            log::error!("unknown client: {}", client_addr);
                        }
                    }
                    _ => (),
                };
            }
        }
    });

    Ok(tx)
}
