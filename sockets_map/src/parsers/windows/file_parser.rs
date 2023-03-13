use std::{
    io::{BufReader, Read, Seek},
    path::PathBuf,
};

use crate::{host::Host, parsers::windows::parse_ip_command_output};
use utf16_reader;

use super::WindowsHostRawData;

pub struct WindowsHostFiles {
    hostname: String,
    network_output_file: PathBuf,
    ip_output_file: PathBuf,
    tasklist_output_file: PathBuf,
}

impl WindowsHostFiles {
    pub fn new(
        hostname: String,
        network_output_file: PathBuf,
        ip_output_file: PathBuf,
        tasklist_output_file: PathBuf,
    ) -> Self {
        Self {
            hostname,
            network_output_file,
            ip_output_file,
            tasklist_output_file,
        }
    }
}

impl From<WindowsHostFiles> for anyhow::Result<WindowsHostRawData> {
    fn from(windows_host_files: WindowsHostFiles) -> Self {
        log::debug!(
            "Parsing netstat, tasklist and get-netipaddress commands output for host {}",
            &windows_host_files.hostname
        );

        // Parse the output of the Get-NetIpAddress command to get the host IPs
        let ip_file = std::fs::File::open(&windows_host_files.ip_output_file)?;

        // Try to read as utf8, otherwise as utf16
        let mut reader = BufReader::new(ip_file);
        let mut buffer = String::new();
        let ip_command_output_file_contents = if reader.read_to_string(&mut buffer).is_ok() {
            buffer
        } else {
            reader.rewind()?;
            utf16_reader::read_to_string(reader)
        };
        let ips = match parse_ip_command_output(
            ip_command_output_file_contents,
            &windows_host_files.hostname,
        ) {
            Ok(ips) => ips,
            Err(e) => return Err(e),
        };

        // Read tasklist file
        let tasklist_file = std::fs::File::open(windows_host_files.tasklist_output_file)?;
        let mut reader = BufReader::new(&tasklist_file);
        // Try to read as utf8, otherwise as utf16
        let mut buffer = String::new();
        let tasklist_output = if reader.read_to_string(&mut buffer).is_ok() {
            buffer
        } else {
            reader.rewind()?;
            utf16_reader::read_to_string(reader)
        };

        // Read netstat file
        let netstat_file = std::fs::File::open(windows_host_files.network_output_file)?;

        // Try to read netstat command output contents as utf8, otherwise as utf16
        let mut reader = BufReader::new(netstat_file);
        let mut buffer = String::new();
        let network_output = if reader.read_to_string(&mut buffer).is_ok() {
            buffer
        } else {
            reader.rewind()?;
            utf16_reader::read_to_string(reader)
        };

        Ok(WindowsHostRawData {
            hostname: windows_host_files.hostname,
            network_output,
            tasklist_output,
            ips,
        })
    }
}

impl From<WindowsHostFiles> for anyhow::Result<Host> {
    fn from(windows_host_files: WindowsHostFiles) -> Self {
        let host_data: anyhow::Result<WindowsHostRawData> = windows_host_files.into();
        host_data.and_then(|h| h.into())
    }
}
