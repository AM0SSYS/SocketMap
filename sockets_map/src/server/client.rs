use std::net::IpAddr;

use crate::{
    host::Host,
    parsers::{linux::LinuxHostRawData, windows::WindowsHostRawData},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum HostData {
    LinuxHostData(LinuxHostRawData),
    WindowsHostData(WindowsHostRawData),
}

impl From<HostData> for anyhow::Result<Host> {
    fn from(host_data: HostData) -> Self {
        match host_data {
            HostData::LinuxHostData(h) => h.into(),
            HostData::WindowsHostData(h) => h.into(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Update {
    pub host: Host,
}

impl Update {
    pub fn new(host: Host) -> Self {
        Self { host }
    }
}

/// This structure stores the information that is passed from the clients to the server.
/// It is to be transformed into a Host structure in order to use the connection model on it.
#[derive(Debug)]
pub struct Client {
    pub hostname: String,
    pub pretty_name: Option<String>,
    /// List of local IPs on the client
    pub ips: Vec<IpAddr>,

    /// Number of updates given by the client
    updates: Vec<Update>,
}

impl Client {
    pub fn new(hostname: String, pretty_name: Option<String>, ips: Vec<IpAddr>) -> Self {
        Self {
            ips,
            updates: vec![],
            hostname,
            pretty_name,
        }
    }

    pub fn add_update(&mut self, update: Update) {
        self.updates.push(update);
    }

    pub fn updates(&self) -> &[Update] {
        self.updates.as_ref()
    }
}
