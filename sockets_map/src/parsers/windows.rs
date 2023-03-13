//! This modules parses the output of the ipconfig and netsat commands to retrieve information from
//! Windows systems.

pub mod agent_parser;
pub mod file_parser;

use crate::host::{self, Host, ListeningSocket, Process, SocketType};
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WindowsHostRawData {
    hostname: String,
    network_output: String,
    tasklist_output: String,
    ips: Vec<IpAddr>,
}

impl WindowsHostRawData {
    pub fn new(
        hostname: String,
        network_output: String,
        tasklist_output: String,
        ips: Vec<IpAddr>,
    ) -> Self {
        Self {
            hostname,
            network_output,
            tasklist_output,
            ips,
        }
    }
}

/// Parse tasklist output
/// Returns a hashmap with (<pid>, "<process name>")
fn parse_tasklist_command_output(
    tasklist_command_output_file_contents: String,
) -> anyhow::Result<HashMap<u32, String>> {
    let mut hashmap = HashMap::<u32, String>::new();

    let lines = tasklist_command_output_file_contents.lines();
    for line in lines {
        // Skip empty lines
        if line.is_empty() {
            continue;
        }
        // Skip header line
        let mut has_digit = false;
        for c in line.chars() {
            if c.is_numeric() {
                has_digit = true;
                break;
            }
        }
        if !has_digit {
            continue;
        }

        let line = line.replace('"', "");
        log::debug!("tasklist line: {}", line);
        let mut line_split = line.split(',');
        let Some(process_name) = line_split.next() else { continue };
        log::debug!("process name: {}", process_name);
        let Some(process_pid) = line_split.next() else { continue };
        log::debug!("pid: {}", process_pid);

        hashmap
            .entry(match process_pid.parse() {
                Ok(p) => p,
                Err(_) => continue,
            })
            .or_insert_with(|| process_name.to_string());
    }

    Ok(hashmap)
}

/// Parse ip command output and add
fn parse_ip_command_output(
    ip_command_output_contents: String,
    hostname: &str,
) -> anyhow::Result<Vec<IpAddr>> {
    let lines = ip_command_output_contents.lines();
    let mut ips = Vec::<IpAddr>::new();
    for line in lines {
        if line.starts_with("IPAddress") {
            // Clean line
            let mut trimmed_line = line.replace("  ", " ");
            while trimmed_line.contains("  ") {
                trimmed_line = trimmed_line.replace("  ", " ");
            }

            // Trim link-local addresses
            if trimmed_line.contains('%') {
                trimmed_line = match trimmed_line.split('%').next() {
                    Some(t) => t,
                    None => continue,
                }
                .to_string();
            }

            let Ok(ip_addr) = match trimmed_line.split(' ').nth(2) {
                Some(s) => s,
                None => continue,
            }
            .parse() else { continue };
            log::debug!("adding ip {} to host {}", ip_addr, hostname);
            ips.push(ip_addr)
        }
    }
    Ok(ips)
}

fn parse_netstat_contents(
    netstat_command_output_file_contents: String,
    process_name_pid_hashmap: std::collections::HashMap<u32, String>,
    host: &mut Host,
) {
    let lines = netstat_command_output_file_contents.lines();

    // Iterate over lines
    for line in lines {
        // Clean line
        let mut trimmed_line = line.replace("  ", " ").trim_start().to_string();
        while trimmed_line.contains("  ") {
            trimmed_line = trimmed_line.replace("  ", " ");
        }
        let line = trimmed_line;

        // Parse TCP lines
        if line.starts_with("TCP") {
            log::debug!("line: {}", line);
            let mut line_split = line.split(' ');
            let Some(local_socket_str) = line_split.clone().nth(1) else { continue };
            log::debug!("local_socket_str: {}", local_socket_str);
            let Some(peer_socket_str) = line_split.clone().nth(2) else { continue };
            log::debug!("peer_socket_str: {}", peer_socket_str);
            let Some(state) = line_split.clone().nth(3) else { continue };
            log::debug!("state: {}", state);
            let pid: u32 = match match line_split.nth(4) {
                Some(l) => l,
                None => continue,
            }
            .parse()
            {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Find process name
            let process_name = match process_name_pid_hashmap.get(&pid) {
                Some(p) => p,
                None => {
                    log::warn!("unable to find process name for PID {}, skipping", pid);
                    continue;
                }
            };
            let process = Process::new(process_name, pid, host.name().to_string());

            match state {
                // Parse established connections
                "ESTABLISHED" => {
                    let local_socket: std::net::SocketAddr = match local_socket_str.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let peer_socket: std::net::SocketAddr = match peer_socket_str.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let connection =
                        host::Connection::new(local_socket, peer_socket, SocketType::TCP, process);
                    host.add_established_connection(connection);
                }
                // Parse listening connections
                "LISTENING" => {
                    let local_socket: std::net::SocketAddr = match local_socket_str.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let ipv6_only = true; // Seems not to exist on Windows, not sure about that though
                    let listening_socket = ListeningSocket::new(
                        local_socket,
                        SocketType::TCP,
                        process,
                        host.name().to_string(),
                        match local_socket.is_ipv6() {
                            true => Some(ipv6_only),
                            false => None,
                        },
                    );
                    host.add_listening_socket(listening_socket);
                }
                _ => {}
            }
        }
    }
}

impl From<WindowsHostRawData> for anyhow::Result<Host> {
    /// Parse the output of the netstat and ip command address command to get the host IPs
    /// The file contains the concatenation of the outputs of the following commands :
    ///
    /// ```bash
    /// Get-NetIpAddress
    /// netstat -p tcp -ano
    /// tasklist /FO CSV
    /// ```
    fn from(host_data: WindowsHostRawData) -> Self {
        log::debug!(
            "Parsing netstat, tasklist and get-netipaddress commands output for host {}",
            host_data.hostname
        );
        let mut host = Host::new(&host_data.hostname);

        // Add IPs
        host_data.ips.iter().for_each(|ip| host.add_ip(*ip));

        // Parse process list
        let process_name_pid_hashmap =
            match parse_tasklist_command_output(host_data.tasklist_output) {
                Ok(h) => h,
                Err(e) => return Err(e),
            };

        // Parse netstat output contents
        parse_netstat_contents(
            host_data.network_output,
            process_name_pid_hashmap,
            &mut host,
        );

        Ok(host)
    }
}
