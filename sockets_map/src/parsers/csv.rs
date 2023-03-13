//! This module handles the retrievl=al of information from a specifically crafted CSV files.

use crate::host;
use anyhow::{bail, Context};
use csv;
use log;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
enum ConState {
    Established,
    Listening,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
struct Record {
    protocol: host::SocketType,
    local_socket: std::net::SocketAddr,
    foreign_socket: Option<std::net::SocketAddr>,
    state: ConState,
    pid: u32,
    process_name: String,
}

impl Record {
    /// Get a reference to the record's protocol.
    fn protocol(&self) -> &host::SocketType {
        &self.protocol
    }

    /// Get a reference to the record's local socket.
    fn local_socket(&self) -> &std::net::SocketAddr {
        &self.local_socket
    }

    /// Get a reference to the record's state.
    fn state(&self) -> &ConState {
        &self.state
    }

    /// Get the record process
    fn process(&self, host_name: &str) -> host::Process {
        host::Process::new(&self.process_name, self.pid, host_name.to_string())
    }

    /// Get a reference to the record's foreign socket.
    fn foreign_socket(&self) -> Option<&std::net::SocketAddr> {
        self.foreign_socket.as_ref()
    }
}

impl host::Host {
    /// Parse a user defined CSV file.
    /// The network file contains the following columns:
    ///
    /// - protocol (tcp, udp, ...)
    /// - local socket
    /// - foreign socket
    /// - state
    /// - pid (not really important, only used when making a summary of the connections)
    /// - process name
    ///
    /// IPv6 sockets format must match what has been defined in RFC2732
    ///
    /// The IP file only has an IP column.
    pub fn from_csv_files(
        hostname: &str,
        network_csv_file_path: std::path::PathBuf,
        ip_csv_file_path: std::path::PathBuf,
    ) -> anyhow::Result<Self> {
        log::debug!("Parsing CSV file for host {}", hostname);
        let mut host = host::Host::new(hostname);

        // Parse IP file
        let mut ip_csv_reader = csv::ReaderBuilder::new()
            .from_path(ip_csv_file_path)
            .with_context(|| "unable to read IP file")?;
        for ip_record in ip_csv_reader.records().flatten() {
            if let Some(ip_record) = ip_record.get(0) {
                host.add_ip(
                    ip_record
                        .parse()
                        .with_context(|| "unable to parse IP CSV file")?,
                );
            } else {
                bail!("error in IP CSV file format");
            }
        }

        // Parse network file
        let mut network_csv_reader = csv::ReaderBuilder::new()
            .from_path(network_csv_file_path)
            .with_context(|| "unable to read network file")?;
        for network_record in network_csv_reader.deserialize() {
            let record: Record = match network_record {
                Ok(n) => n,
                Err(e) => {
                    log::warn!("unable to parse CSV network record: {}", e);
                    continue;
                }
            };
            match record.state() {
                ConState::Established => {
                    if let Some(foreign_socket) = record.foreign_socket() {
                        host.add_established_connection(host::Connection::new(
                            *record.local_socket(),
                            *foreign_socket,
                            record.protocol().clone(),
                            record.process(hostname).clone(),
                        ));
                    } else {
                        log::warn!("missing foreign socket for connection");
                        continue;
                    }
                }
                ConState::Listening => {
                    let ipv6_only = match record.local_socket().is_ipv6() {
                        true => Some(
                            record.local_socket().to_string().starts_with('*')
                                || record.local_socket().to_string().starts_with("[::ffff:"),
                        ),
                        false => None,
                    };
                    host.add_listening_socket(host::ListeningSocket::new(
                        *record.local_socket(),
                        record.protocol().clone(),
                        record.process(hostname),
                        hostname.to_string(),
                        ipv6_only,
                    ));
                }
            }
        }

        Ok(host)
    }
}
