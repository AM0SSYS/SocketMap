//! This module represents hosts with their processes and connections.

use hex;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use std::{net::IpAddr, vec};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
/// A process that can be linked to some sockets
pub struct Process {
    /// The process name
    name: String,
    /// The process ID
    pid: u32,
    /// Its unique node id to be graphically represented
    node_id: String,
}

impl Process {
    pub fn new(name: &str, pid: u32, host_name: String) -> Self {
        Self {
            name: name.to_string(),
            pid,
            node_id: format!("{host_name}_{name}").replace(['.', '?', '-'], "_"),
        }
    }

    #[allow(dead_code)]
    /// Get a reference to the process's pid.
    pub fn pid(&self) -> &u32 {
        &self.pid
    }

    /// Get a reference to the process's node id.
    pub fn node_id(&self) -> &str {
        self.node_id.as_str()
    }

    /// Get a reference to the process's name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord)]
/// A listening socket wich is linked to its parent process
pub struct ListeningSocket {
    socket: std::net::SocketAddr,
    /// TCP, UDP, ...
    socket_type: SocketType,
    process: Process,
    /// A listening socket ends up being a node, so it needs a humanly readable name
    node_name: String,
    /// It also needs a unique node ID, with is a hash hex string made from its name
    node_id: String,
    /// Whether the socket also accepts IP6 connections or not
    ipv6_only: Option<bool>,
}

impl ListeningSocket {
    pub fn new(
        socket: std::net::SocketAddr,
        socket_type: SocketType,
        process: Process,
        host_name: String,
        ipv6_only: Option<bool>,
    ) -> Self {
        let ip_version = match socket.is_ipv4() {
            true => 4,
            false => 6,
        };
        let node_name = format!(
            "{}\n{}{}:{}",
            process.name(),
            match socket_type {
                SocketType::TCP => "tcp",
                SocketType::UDP => "udp",
                SocketType::UNIX => "unix",
            },
            {
                match ipv6_only {
                    Some(b) => {
                        if !b {
                            "4/6".to_string()
                        } else {
                            ip_version.to_string()
                        }
                    }
                    None => ip_version.to_string(),
                }
            },
            socket.port()
        );

        // Build node id by making the hash of a string containing the node main attributes
        // The hash has to be prepended by a letter (here 'a' because the node ID validation regex
        // wants a char before a number)
        let node_id_str = format!("{}_{}_{}", host_name, process.name(), socket.port());
        let mut hasher = sha1::Sha1::new();
        hasher.update(node_id_str);
        let node_id_vec = hasher.finalize().to_ascii_uppercase();
        let mut node_id = String::from("a");
        node_id.push_str(&hex::encode(node_id_vec));
        log::debug!("node id: {:?}", node_id);

        Self {
            socket,
            socket_type,
            process,
            node_name,
            node_id,
            ipv6_only,
        }
    }

    /// Get a reference to the listening tcp socket's ip addr.
    pub fn ip_addr(&self) -> std::net::IpAddr {
        self.socket.ip()
    }

    /// Get a reference to the listening tcp socket's port.
    pub fn port(&self) -> u16 {
        self.socket.port()
    }

    /// Get a reference to the listening tcp socket's node name.
    pub fn node_name(&self) -> &str {
        self.node_name.as_str()
    }

    /// Get a reference to the listening socket's socket type.
    pub fn socket_type(&self) -> &SocketType {
        &self.socket_type
    }

    /// Get a reference to the listening socket's node id.
    pub fn node_id(&self) -> &str {
        self.node_id.as_str()
    }

    /// Returns true if this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        self.socket.ip().is_loopback()
    }

    /// Get a reference to the listening socket's process.
    pub fn process(&self) -> &Process {
        &self.process
    }

    /// Get a reference to the listening socket's socket.
    pub fn socket(&self) -> &std::net::SocketAddr {
        &self.socket
    }

    /// Get a reference to the listening socket's ipv6 only.
    pub fn ipv6_only(&self) -> Option<&bool> {
        self.ipv6_only.as_ref()
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum SocketType {
    TCP,
    UDP,
    UNIX,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
/// A connection between two sockets, and hence between two processes
pub struct Connection {
    /// TCP, UDP, ...
    socket_type: SocketType,
    local_socket: std::net::SocketAddr,
    peer_socket: std::net::SocketAddr,
    /// The parent process of the local socket
    process: Process,
}

impl Connection {
    pub fn new(
        local_socket: std::net::SocketAddr,
        peer_socket: std::net::SocketAddr,
        socket_type: SocketType,
        process: Process,
    ) -> Self {
        Self {
            socket_type,
            local_socket,
            peer_socket,
            process,
        }
    }

    /// Get a reference to the connection's socket type.
    pub fn socket_type(&self) -> &SocketType {
        &self.socket_type
    }

    /// Get a reference to the connection's local socket.
    pub fn local_socket(&self) -> &std::net::SocketAddr {
        &self.local_socket
    }

    /// Get a reference to the connection's peer socket.
    pub fn peer_socket(&self) -> &std::net::SocketAddr {
        &self.peer_socket
    }

    /// Get a reference to the connection's process.
    pub fn process(&self) -> &Process {
        &self.process
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// A host that has processes and connections
pub struct Host {
    /// Unique host name
    name: String,
    /// A host has a cluster id in order to graphically group its processes around itself
    cluster_id: String,
    /// The host's listening sockets / processes
    listening_sockets: Vec<ListeningSocket>,
    /// Host connections between processes (loopback or not)
    connections: Vec<Connection>,
    /// IP addresses associated with the host
    ips: Vec<IpAddr>,
}

impl Host {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cluster_id: format!("cluster_{name}").replace('-', "_"),
            listening_sockets: Vec::new(),
            connections: Vec::new(),
            ips: vec!["127.0.0.1".parse().unwrap(), "::1".parse().unwrap()],
        }
    }

    pub fn add_listening_socket(&mut self, s: ListeningSocket) {
        log::debug!(
            "add listening socket {}:{} to {} with ipv6_only={}",
            s.ip_addr(),
            s.port(),
            self.name(),
            match s.ipv6_only() {
                Some(b) => b.to_string(),
                None => "none".to_string(),
            }
        );
        self.listening_sockets.push(s);
    }

    pub fn add_established_connection(&mut self, c: Connection) {
        log::debug!(
            "add established connection between {} and {}",
            c.local_socket(),
            c.peer_socket()
        );
        self.connections.push(c);
    }

    pub fn add_ip(&mut self, ip: IpAddr) {
        log::debug!("add IP {} to {}", ip, self.name);
        self.ips.push(ip);
        // Also put the IP6 equivalent such as [::ffff:127.0.0.1]
        if ip.is_ipv4() {
            let ip_str = ip.to_string();
            let ip6_addr: std::net::Ipv6Addr = format!("::ffff:{ip_str}").parse().unwrap();
            self.ips.push(std::net::IpAddr::from(ip6_addr));
        }
    }

    /// Get a reference to the host's listening sockets.
    pub fn listening_sockets(&self) -> &[ListeningSocket] {
        &self.listening_sockets
    }

    pub fn listening_sockets_mut(&mut self) -> &mut Vec<ListeningSocket> {
        &mut self.listening_sockets
    }

    /// Get a reference to the host's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a reference to the host's connections.
    pub fn connections(&self) -> &[Connection] {
        self.connections.as_slice()
    }

    pub fn connections_mut(&mut self) -> &mut Vec<Connection> {
        &mut self.connections
    }

    /// Get a reference to the host's ips.
    pub fn ips(&self) -> &[IpAddr] {
        self.ips.as_slice()
    }

    /// Get a reference to the host's cluster id.
    pub fn cluster_id(&self) -> &str {
        self.cluster_id.as_str()
    }

    /// Filter out the connections on process name matching
    pub fn exclude_processes(&mut self, pattern: &[&str]) {
        self.connections
            .retain(|c| pattern.iter().any(|p| !c.process.name.starts_with(p)));
    }
}
