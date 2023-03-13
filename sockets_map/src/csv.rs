//! This module handles the CSV output of the connections graph.

use crate::connections_model::Connection;
use crate::host;
use anyhow::Context;
use csv;

/// Output a CSV formatted string of all the hosts connections with the following columns :
///
/// - Source host
/// - Dest host
/// - Source process name
/// - Source process PID
/// - Dest process name
/// - Dest process PID
/// - Source socket
/// - Dest socket
///
pub fn write_connections_to_csv(
    connections: &Vec<Connection>,
    out_file_path: &std::path::Path,
) -> anyhow::Result<()> {
    let out_file = match std::fs::File::create(out_file_path) {
        Ok(f) => f,
        Err(e) => {
            log::error!("unable to create file {:?}: {}", out_file_path, e);
            std::process::exit(1);
        }
    };

    let mut wtr = csv::Writer::from_writer(&out_file);

    wtr.write_record([
        "Source host",
        "Dest host",
        "Source process",
        "Dest process",
        "Source PID",
        "Dest PID",
        "Source process socket",
        "Dest process socket",
        "Protocol",
    ])
    .with_context(|| "unable to write CSV records to file")?;

    for conn in connections {
        let connected_host_name = conn.connected_host().name();
        let listening_host_name = conn.listening_host().name();
        let source_process_name = conn.connected_connection().process().name();
        let source_process_pid = conn.connected_connection().process().pid();
        let listening_process_name = conn.listening_connection().process().name();
        let listening_process_pid = conn.listening_connection().process().pid();
        let source_socket = conn.connected_connection().local_socket().to_string();
        let dest_socket = conn.listening_connection().socket().to_string();
        let protocol = match conn.connected_connection().socket_type() {
            host::SocketType::TCP => "TCP",
            host::SocketType::UDP => "UDP",
            host::SocketType::UNIX => "UNIX",
        };

        wtr.write_record([
            connected_host_name,
            listening_host_name,
            source_process_name,
            listening_process_name,
            source_process_pid.to_string().as_str(),
            listening_process_pid.to_string().as_str(),
            &source_socket,
            &dest_socket,
            protocol,
        ])
        .with_context(|| "unable to write CSV records to file")?;
    }
    Ok(())
}
