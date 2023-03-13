use super::{client::Update, host};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Message {
    Register(Register),
    Update(Update),
    UpdateRequest,
    StartRecording(f64),
    StopRecording,
    Exit,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Register {
    hostname: String,
    pretty_name: Option<String>,
    ip_addresses: Vec<IpAddr>,
}

impl Register {
    pub fn new(hostname: String, pretty_name: Option<String>, ip_addresses: Vec<IpAddr>) -> Self {
        Self {
            hostname,
            pretty_name,
            ip_addresses,
        }
    }

    pub fn hostname(&self) -> &str {
        self.hostname.as_ref()
    }

    pub fn pretty_name(&self) -> Option<&String> {
        self.pretty_name.as_ref()
    }

    pub fn ip_addresses(&self) -> &[IpAddr] {
        self.ip_addresses.as_ref()
    }
}

/// The process structure that will be passed from the agents to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Process {
    name: String,
    pid: u32,
}

/// The socket structure that will be passed from the agents to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Socket {
    socket_str: String,
    socket_type: host::SocketType,
    process: host::Process,
    ipv6_only: Option<bool>,
}

impl Socket {
    pub fn socket_str(&self) -> &str {
        self.socket_str.as_ref()
    }

    pub fn socket_type(&self) -> &host::SocketType {
        &self.socket_type
    }

    pub fn process(&self) -> &host::Process {
        &self.process
    }

    pub fn ipv6_only(&self) -> Option<bool> {
        self.ipv6_only
    }
}
