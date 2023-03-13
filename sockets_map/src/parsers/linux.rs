//! This modules parses the output of the ss, ip and netsat commands to retrieve information from
//! Linux systems.

pub mod agent_parser;
pub mod file_parser;

use crate::host::{self, Host};
use anyhow::anyhow;
use log;
use regex;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum NetworkOutput {
    Ss(String),
    Netstat(String),
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LinuxHostRawData {
    hostname: String,
    network_output: NetworkOutput,
    ips: Vec<IpAddr>,
}

impl LinuxHostRawData {
    pub fn new(hostname: String, network_output: NetworkOutput, ips: Vec<IpAddr>) -> Self {
        Self {
            hostname,
            network_output,
            ips,
        }
    }
}

pub fn parse_netstat_contents(
    lines: std::str::Lines,
    host: &mut Host,
    mut warned_about_malformed_lines: bool,
) -> Option<anyhow::Result<Host>> {
    for line in lines {
        // Skip lines that do not concern LISTENING OR ESTABLISHED connections
        if !(line.contains("ESTABLISHED") || line.contains("LISTEN")) {
            continue;
        }
        log::debug!("netstat line: {}", line);

        // Split line into columns
        let mut trimmed_line = line.to_string();
        while trimmed_line.contains("  ") {
            trimmed_line = trimmed_line.replace("  ", " ");
        }
        let split_line: Vec<&str> = trimmed_line.split(' ').collect();

        // Get protocol
        let Some(protocol) = split_line.first() else { continue };
        log::debug!("protocol: {}", protocol);
        // Get local socket
        let Some(local_socket_str) = split_line.get(3) else { continue };
        log::debug!("local_socket: {}", local_socket_str);
        // Get peer socket
        let Some(peer_socket_str) = split_line.get(4) else { continue };
        log::debug!("peer_socket: {}", peer_socket_str);
        // Get state
        let Some(state) = split_line.get(5) else { continue };
        log::debug!("state: {}", state);
        // Get process name
        let process_info = match split_line.get(6) {
            Some(p) => p,
            None => {
                if !warned_about_malformed_lines {
                    warned_about_malformed_lines = true;
                    log::warn!("Some lines of the netstat output do not contain the process name. This can be normal for some lines, but it can also be because the command was not ran as root. If you're sure you did, you can ignore this warning.");
                }
                continue;
            }
        };
        log::debug!("process_info: {}", process_info);

        let process_pid: u32 = match match process_info.split('/').next() {
            Some(s) => s,
            None => continue,
        }
        .parse()
        {
            Ok(s) => s,
            Err(_) => continue,
        };
        log::debug!("process_pid: {}", process_pid);
        let Some(process_name) = process_info.split('/').nth(1) else { continue };
        log::debug!("process_name: {}", process_name);
        let process = host::Process::new(process_name, process_pid, host.name().to_string());

        // IPv6
        let ipv6 = protocol.ends_with('6');
        let ipv6_only = true; // the netstat command does not indicate whether the socket is ipv6only or not, so we assume it is in order not to miss connections

        // Parse socket here to deal with the netstat formatting of IPv6 sockets issue
        // If the parsing succeeds, it's an IPv4 address, if not, it is an IPv6 one and needs
        // formatting
        let local_socket: std::net::SocketAddr = match local_socket_str.parse() {
            Ok(l) => l,
            Err(_) => {
                let mut local_socket_str_split = local_socket_str.split(':');
                let port = match local_socket_str_split.nth_back(0) {
                    Some(p) => p,
                    None => continue,
                };
                let last_colon_index = match local_socket_str.rfind(':') {
                    Some(l) => l,
                    None => continue,
                };
                let address = local_socket_str[0..last_colon_index].to_string();
                let local_socket_str = format!("[{address}]:{port}");
                log::debug!(
                    "reformatted netstat IPv6 local_socket: {}",
                    local_socket_str
                );
                match peer_socket_str.parse() {
                    Ok(p) => p,
                    Err(e) => return Some(Err(anyhow!("unable to parse IPv6 peer socket: {e}"))),
                }
            }
        };

        // Socket type
        let socket_type: host::SocketType = match *protocol {
            "tcp" | "tcp6" => host::SocketType::TCP,
            "udp" | "udp6" => host::SocketType::UDP,
            _ => {
                continue;
            }
        };

        match *state {
            "ESTABLISHED" => {
                // Same as for the local socket parsing
                let peer_socket: std::net::SocketAddr = match peer_socket_str.parse() {
                    Ok(l) => l,
                    Err(_) => {
                        let mut peer_socket_str_split = peer_socket_str.split(':');
                        let Some(port) = peer_socket_str_split.nth_back(0) else { continue };
                        let Some(last_colon_index) = peer_socket_str.rfind(':') else { continue };
                        let address = peer_socket_str[0..last_colon_index].to_string();
                        let peer_socket_str = format!("[{address}]:{port}");
                        log::debug!("reformatted netstat IPv6 peer_socket: {}", peer_socket_str);
                        match peer_socket_str.parse() {
                            Ok(p) => p,
                            Err(e) => {
                                return Some(Err(anyhow!("unable to parse IPv6 peer socket: {e}")))
                            }
                        }
                    }
                };
                // Create established connection
                let connection =
                    host::Connection::new(local_socket, peer_socket, socket_type, process);
                host.add_established_connection(connection);
            }
            "LISTEN" => {
                // Create listening connection
                let listening_socket = host::ListeningSocket::new(
                    local_socket,
                    socket_type,
                    process,
                    host.name().to_string(),
                    match ipv6 {
                        true => Some(ipv6_only),
                        false => None,
                    },
                );
                host.add_listening_socket(listening_socket);
            }
            _ => {
                continue;
            }
        };
    }
    None
}

pub fn parse_ss_contents(
    lines: std::str::Lines,
    host: &mut Host,
    warned_about_malformed_lines: &mut bool,
) {
    for line in lines {
        // Cleanup line by removing extraneous whitespaces
        let split_line = clean_and_split_line(line);

        // Parse TCP and UDP socktets
        if line.starts_with("tcp") | line.starts_with("udp") {
            // Socket type and state
            let Some(socket_str) = split_line.get(0) else { continue };
            let Some(state) = split_line.get(1) else { continue };
            let socket_type = match &socket_str[..] {
                "udp" => host::SocketType::UDP,
                "tcp" => host::SocketType::TCP,
                _ => continue,
            };

            // Listening TCP
            if state == "LISTEN" && socket_type == host::SocketType::TCP {
                // Parse this line as a listening socket
                let listening_socket = parse_listening_socket_ss_line(
                    &split_line,
                    host.name(),
                    socket_type.clone(),
                    warned_about_malformed_lines,
                );
                if let Some(l) = listening_socket {
                    host.add_listening_socket(l);
                }
            }

            // Established TCP
            let Some(state) = split_line.get(1) else { continue };
            if state == "ESTAB" && socket_type == host::SocketType::TCP {
                // Create the Connection struct and add it to the Host
                let established_connection = parse_established_connection_ss_line(
                    &split_line,
                    host.name(),
                    socket_type.clone(),
                    warned_about_malformed_lines,
                );
                if let Some(c) = established_connection {
                    host.add_established_connection(c);
                }
            }

            // Listening UDP
            if state == "UNCONN" && socket_type == host::SocketType::UDP {
                // Parse this line as an established connection socket
                let listening_socket = parse_listening_socket_ss_line(
                    &split_line,
                    host.name(),
                    socket_type.clone(),
                    warned_about_malformed_lines,
                );
                if let Some(l) = listening_socket {
                    host.add_listening_socket(l);
                }
            }

            // Established UDP
            if state == "ESTAB" && socket_type == host::SocketType::UDP {
                // Parse this line as an established connection socket
                let established_connection = parse_established_connection_ss_line(
                    &split_line,
                    host.name(),
                    socket_type.clone(),
                    warned_about_malformed_lines,
                );
                if let Some(c) = established_connection {
                    host.add_established_connection(c);
                }
            }
        }
    }
}

/// Cleanup line by removing extraneous whitespaces and return a split
fn clean_and_split_line(line: &str) -> Vec<String> {
    let mut trimmed_line = line.to_string();
    while trimmed_line.contains("  ") {
        trimmed_line = trimmed_line.replace("  ", " ");
    }
    trimmed_line = trimmed_line.trim().to_string();
    let split_line: Vec<String> = trimmed_line.split(' ').map(|s| s.to_string()).collect();

    split_line
}

/// Parse a listening socket ss line
fn parse_listening_socket_ss_line(
    split_line: &[String],
    hostname: &str,
    socket_type: host::SocketType,
    warned_about_malformed_lines: &mut bool,
) -> Option<host::ListeningSocket> {
    // Get sockets
    let Some(local_socket_str) = split_line.get(4) else { return None };
    log::debug!("local_socket_str: {}", local_socket_str);

    // Clean loopback sockets from the "%iface" subststring, like in "127.0.0.53%lo:53"
    let re = regex::Regex::new(r"%\w+:").unwrap();
    let local_socket_str = re.replace(local_socket_str, ":");

    // Process
    let process_info = match split_line.get(6) {
        Some(p) => p,
        None => {
            if !*warned_about_malformed_lines {
                *warned_about_malformed_lines = true;
                log::warn!("Some lines of the ss output do not contain the process name. This can be normal for some lines, but it can also be because the command was not ran as root. If you're sure you did, you can ignore this warning.");
            }
            return None;
        }
    };
    let Some(process_name) = process_info.split('"').nth(1) else { return None };
    let pid: u32 = match match match process_info.split(',').nth(1) {
        Some(s) => s,
        None => return None,
    }
    .split('=')
    .nth(1)
    {
        Some(s) => s,
        None => return None,
    }
    .parse()
    {
        Ok(p) => p,
        Err(_) => return None,
    };
    let process = host::Process::new(process_name, pid, hostname.to_string());

    // IPv6
    let ipv6 = local_socket_str.starts_with('[') || local_socket_str.starts_with('*');
    // * and [::] indicate whether the IPV6_V6ONLY flag was set to false or true during socket creation, respectively
    let ipv6_only = match ipv6 {
        true => {
            Some(!(local_socket_str.starts_with('*') || local_socket_str.starts_with("[::ffff:")))
        }
        false => None,
    };

    // Create the ListeningSocket struct and add it to the Host
    let local_socket: std::net::SocketAddr = match match ipv6 {
        true => local_socket_str.replace('*', "[::]").parse(),
        false => local_socket_str.parse(),
    } {
        Ok(l) => l,
        Err(_) => return None,
    };

    Some(host::ListeningSocket::new(
        local_socket,
        socket_type,
        process,
        hostname.to_string(),
        ipv6_only,
    ))
}

/// Parse an established connection ss line
fn parse_established_connection_ss_line(
    split_line: &[String],
    hostname: &str,
    socket_type: host::SocketType,
    warned_about_malformed_lines: &mut bool,
) -> Option<host::Connection> {
    // Get sockets
    let Some(local_socket_str) = split_line.get(4) else { return None };
    let Some(peer_socket_str) = split_line.get(5) else { return None };

    // Process
    let process_info = match split_line.get(6) {
        Some(p) => p,
        None => {
            if !*warned_about_malformed_lines {
                *warned_about_malformed_lines = true;
                log::warn!("Some lines of the ss output do not contain the process name. This can be normal for some lines, but it can also be because the command was not ran as root. If you're sure you did, you can ignore this warning.");
            }
            return None;
        }
    };
    let Some(process_name) = process_info.split('"').nth(1) else { return None };
    let pid: u32 = match match match process_info.split(',').nth(1) {
        Some(s) => s,
        None => return None,
    }
    .split('=')
    .nth(1)
    {
        Some(s) => s,
        None => return None,
    }
    .parse()
    {
        Ok(p) => p,
        Err(_) => return None,
    };
    let process = host::Process::new(process_name, pid, hostname.to_string());

    // Create the Connection struct and add it to the Host
    Some(host::Connection::new(
        match local_socket_str.parse() {
            Ok(l) => l,
            Err(_) => return None,
        },
        match peer_socket_str.parse() {
            Ok(l) => l,
            Err(_) => return None,
        },
        socket_type,
        process,
    ))
}

/// Parse IPs from `ip a` command output and add to the host
pub fn parse_ip_command_output(
    ip_command_output_file_contents: String,
) -> anyhow::Result<Vec<IpAddr>> {
    let lines = ip_command_output_file_contents.lines();
    let mut ips = Vec::<IpAddr>::new();

    for line in lines {
        // Clean line by removing indentation
        let line = line.trim();

        // Get IPv4 and IPv6 addresses
        if line.starts_with("inet") {
            let ip_addr: std::net::IpAddr = match match match line.split(' ').nth(1) {
                Some(s) => s,
                None => continue,
            }
            .split('/')
            .next()
            {
                Some(s) => s,
                None => continue,
            }
            .parse()
            {
                Ok(s) => s,
                Err(_) => continue,
            };
            ips.push(ip_addr);
        }
    }
    Ok(ips)
}

impl From<LinuxHostRawData> for anyhow::Result<Host> {
    fn from(host_data: LinuxHostRawData) -> Self {
        log::debug!(
            "Parsing network info and ip commands output for host {}",
            host_data.hostname
        );
        let mut host = Host::new(&host_data.hostname);

        // Add IPs
        for ip in host_data.ips {
            host.add_ip(ip);
        }

        // Parse network command output content
        let network_output_contents = match &host_data.network_output {
            NetworkOutput::Ss(data) => data,
            NetworkOutput::Netstat(data) => data,
        };
        let lines = network_output_contents.lines();
        let mut warned_about_malformed_lines = false;
        match host_data.network_output {
            NetworkOutput::Ss(_) => {
                parse_ss_contents(lines, &mut host, &mut warned_about_malformed_lines);
            }
            NetworkOutput::Netstat(_) => {
                parse_netstat_contents(lines, &mut host, warned_about_malformed_lines);
            }
        }

        Ok(host)
    }
}
