use std::path::PathBuf;

use anyhow::{anyhow, Context};

use crate::{host::Host, parsers::linux::parse_ip_command_output};

use super::{LinuxHostRawData, NetworkOutput};

pub enum NetworkOutputFile {
    Ss(PathBuf),
    Netstat(PathBuf),
}

pub struct LinuxHostFiles {
    hostname: String,
    network_output_file: NetworkOutputFile,
    ip_output_file: PathBuf,
}

impl LinuxHostFiles {
    pub fn new(
        hostname: String,
        network_output_file: NetworkOutputFile,
        ip_output_file: PathBuf,
    ) -> Self {
        Self {
            hostname,
            network_output_file,
            ip_output_file,
        }
    }
}

impl From<LinuxHostFiles> for anyhow::Result<LinuxHostRawData> {
    fn from(linux_host_files: LinuxHostFiles) -> Self {
        let ip_command_output_file_contents =
            std::fs::read_to_string(&linux_host_files.ip_output_file).with_context(|| {
                format!("unable to read file {:?}", linux_host_files.ip_output_file)
            })?;

        // Parse the output of the ip address command to get the host IPs
        let ips = parse_ip_command_output(ip_command_output_file_contents)?;

        let network_file_path = match &linux_host_files.network_output_file {
            NetworkOutputFile::Ss(path) => path,
            NetworkOutputFile::Netstat(path) => path,
        };

        // Check file exists
        if !network_file_path.exists() {
            return Err(anyhow!(
                "file {:?} does not exist",
                network_file_path.to_str()
            ));
        }

        // Read contents into lines
        let network_info_command_output_file_contents = std::fs::read_to_string(network_file_path)
            .with_context(|| format!("unable to read file {network_file_path:?}",))?;

        Ok(LinuxHostRawData {
            hostname: linux_host_files.hostname,
            network_output: match &linux_host_files.network_output_file {
                NetworkOutputFile::Ss(_) => {
                    NetworkOutput::Ss(network_info_command_output_file_contents)
                }
                NetworkOutputFile::Netstat(_) => {
                    NetworkOutput::Netstat(network_info_command_output_file_contents)
                }
            },
            ips,
        })
    }
}

impl From<LinuxHostFiles> for anyhow::Result<Host> {
    fn from(linux_host_files: LinuxHostFiles) -> Self {
        let host_data: anyhow::Result<LinuxHostRawData> = linux_host_files.into();
        host_data.and_then(|h| h.into())
    }
}
