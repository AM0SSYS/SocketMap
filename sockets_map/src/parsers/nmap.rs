//! This modules parses the output of the nmap command to retrieve information from remote machines
//! on which the user could not execute more accurate commands such as ss or netsat.

use crate::host;
use anyhow::{bail, Context};
use log;

impl host::Host {
    /// Parse the output of the nmap command.
    /// The file contains the concatenation of the outputs of the following commands :
    ///
    /// ```bash
    /// nmap <-4|-6> <ip>
    /// ```
    pub fn from_nmap_output_file(
        hostname: &str,
        nmap_output_file_path: std::path::PathBuf,
    ) -> anyhow::Result<Self> {
        log::debug!("Parsing nmap output file for host {}", hostname);
        let mut host = host::Host::new(hostname);

        let nmap_output_file_contents = match std::fs::read_to_string(&nmap_output_file_path) {
            Ok(f) => f,
            Err(e) => {
                bail!(format!(
                    "unable to open file {nmap_output_file_path:?}: {e}"
                ))
            }
        };

        // Parse IP
        let ip_str = match nmap_output_file_path.file_name() {
            Some(f) => f
                .to_string_lossy()
                .split('.')
                .skip(1)
                .collect::<Vec<&str>>()
                .join(".")
                .replace("nmap_", ""),
            None => bail!("unable to get nmap output filename"),
        };
        let ip: std::net::IpAddr = ip_str
            .parse()
            .with_context(|| "unable to parse IP {ip_str}")?;
        host.add_ip(ip);

        // Parse lines
        let lines = nmap_output_file_contents.lines();
        for line in lines {
            // Skip lines that do not start with a number (port)
            match line.chars().next() {
                Some(c) => {
                    if !(c.is_numeric()) {
                        continue;
                    }
                }
                None => continue,
            }

            // Clean line
            let mut trimmed_line = line.replace("  ", " ").trim().to_string();
            while trimmed_line.contains("  ") {
                trimmed_line = trimmed_line.replace("  ", " ");
            }

            // Parse fields
            let mut split_line = trimmed_line.split(' ');
            if let Some(port_proto) = split_line.next() {
                if let Some(state) = split_line.next() {
                    if let Some(service) = split_line.next() {
                        if let Some(port) = port_proto.split('/').next() {
                            if let Some(proto) = port_proto.split('/').nth(1) {
                                log::debug!("nmap line: {}/{} {} {}", port, proto, state, service);

                                let socket: std::net::SocketAddr = match ip.is_ipv4() {
                                    true => format!("{ip_str}:{port}")
                                        .parse::<std::net::SocketAddr>()
                                        .with_context(|| "unable to parse IPv4 address")?,
                                    false => format!("[{ip_str}]:{port}")
                                        .parse::<std::net::SocketAddr>()
                                        .with_context(|| "unable to parse IPv6 address")?,
                                };
                                let socket_type = match proto {
                                    "tcp" => host::SocketType::TCP,
                                    "udp" => host::SocketType::UDP,
                                    _ => continue,
                                };
                                let process = host::Process::new(
                                    format!("{service}?").as_str(),
                                    0,
                                    hostname.to_string(),
                                );
                                log::debug!("new process {}", process.name());

                                let listening_socket = host::ListeningSocket::new(
                                    socket,
                                    socket_type,
                                    process,
                                    hostname.to_string(),
                                    match ip.is_ipv4() {
                                        true => None,
                                        false => Some(true),
                                    },
                                );
                                host.add_listening_socket(listening_socket);
                            }
                        }
                    }
                }
            } else {
                continue;
            }
        }

        Ok(host)
    }
}
