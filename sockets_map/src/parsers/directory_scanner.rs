//! This module scans the samples directory and assumes the type of samples from the files names.

use std::path::Path;

use crate::host::{self, Host};
use anyhow::bail;
use log;

use super::{
    linux::file_parser::{LinuxHostFiles, NetworkOutputFile},
    windows::file_parser::WindowsHostFiles,
};

#[derive(Clone)]
pub enum FileType {
    LinuxIp,
    WindowsIp,
    LinuxNetstat,
    WindowsNetstat,
    WindowsTasklist,
    LinuxSs,
    Nmap,
    CsvIp,
    CsvNetwork,
}

#[derive(Clone)]
pub struct File {
    path: std::path::PathBuf,
    file_type: FileType,
}

impl File {
    pub fn new(path: std::path::PathBuf, file_type: FileType) -> Self {
        Self { path, file_type }
    }

    /// Get a reference to the file's file type.
    pub fn file_type(&self) -> &FileType {
        &self.file_type
    }

    /// Get a reference to the file's path.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Clone)]
pub struct ScannedHost {
    name: String,
    files: Vec<File>,
}

impl ScannedHost {
    fn add_file(&mut self, file: File) {
        self.files.push(file);
    }

    /// Get a reference to the scanned host's name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Get a reference to the scanned host's files.
    pub fn files(&self) -> &[File] {
        self.files.as_slice()
    }
}

impl ScannedHost {
    pub fn new(name: String) -> Self {
        Self {
            name,
            files: Vec::new(),
        }
    }
}

/// Scan a directory looking for files with the following format : `<machine_name>.<file_type>`
/// The file type can be:
///
/// - `ss`
/// - `linux_netsat`
/// - `windows_netsat`
/// - `linux_ip`
/// - `windows_ip`
/// - `nmap_<ip>`
pub fn scan_dir(path: &Path) -> Vec<ScannedHost> {
    let mut scanned_hosts = Vec::<ScannedHost>::new();
    let mut scanned_hosts_names = Vec::<String>::new();

    for entry in path.read_dir().expect("unable to read directory").flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            // Skip directories
            continue;
        }
        log::debug!("seeing {}", entry_path.to_string_lossy());
        let filetype_str = match entry_path.extension() {
            Some(e) => e.to_string_lossy(),
            None => {
                // Skip files without extensions
                continue;
            }
        };

        let filetype = match &filetype_str[..] {
            "ss" => FileType::LinuxSs,
            "linux_netstat" => FileType::LinuxNetstat,
            "windows_netstat" => FileType::WindowsNetstat,
            "windows_ip" => FileType::WindowsIp,
            "linux_ip" => FileType::LinuxIp,
            "windows_tasklist" => FileType::WindowsTasklist,
            _ => {
                // Nmap file are a bit trickier to detect because of the IP at the end
                if let Some(entry_path_filename) = entry_path.file_name() {
                    if entry_path_filename
                        .to_string_lossy()
                        .split('.')
                        .skip(1)
                        .collect::<Vec<&str>>()
                        .join(".")
                        .starts_with("nmap_")
                    {
                        FileType::Nmap
                    }
                    // CSV file have the ip or network str in the stem, not in the extension
                    else if filetype_str == "csv"
                        && match entry_path.file_stem() {
                            Some(s) => s.to_string_lossy().ends_with("_network"),
                            None => false,
                        }
                    {
                        FileType::CsvNetwork
                    } else if filetype_str == "csv"
                        && match entry_path.file_stem() {
                            Some(s) => s.to_string_lossy().ends_with("_ip"),
                            None => false,
                        }
                    {
                        FileType::CsvIp
                    } else {
                        // Skip if extension is unknown
                        log::debug!("skipping file {:?}", entry_path.file_name());
                        continue;
                    }
                } else {
                    continue;
                }
            }
        };
        let hostname = match filetype {
            FileType::Nmap => {
                if let Some(entry_path_filename) = entry_path.file_name() {
                    match entry_path_filename.to_string_lossy().split('.').next() {
                        Some(h) => h.to_string(),
                        None => continue,
                    }
                } else {
                    continue;
                }
            }
            FileType::CsvIp | FileType::CsvNetwork => {
                if let Some(entry_path_filename) = entry_path.file_name() {
                    match entry_path_filename
                        .to_string_lossy()
                        .replace("_ip.", ".")
                        .replace("_network.", ".")
                        .split('.')
                        .next()
                    {
                        Some(h) => h.to_string(),
                        None => continue,
                    }
                } else {
                    continue;
                }
            }
            _ => match entry_path.file_stem() {
                Some(h) => h.to_string_lossy().to_string(),
                None => {
                    // Skip files without stem
                    continue;
                }
            },
        };
        log::debug!("found hostname {}", hostname);

        let file = File::new(entry_path.clone(), filetype);

        // Check if we have seen that host previously
        if !scanned_hosts_names.contains(&hostname) {
            scanned_hosts_names.push(hostname.clone());
            let host = ScannedHost::new(hostname.clone());
            scanned_hosts.push(host);
        }

        // Find the corresponding host in the list
        let mut host: Option<&mut ScannedHost> = None;
        for h in &mut scanned_hosts {
            if h.name == hostname.clone() {
                host = Some(h);
            }
        }

        // Add the file to it, if we can find the host
        match host {
            Some(h) => h.add_file(file),
            None => continue,
        };
    }

    scanned_hosts
}

/// Build the hosts vector
pub fn build_hosts(scanned_hosts: &[ScannedHost]) -> anyhow::Result<Vec<host::Host>> {
    let mut hosts = Vec::<host::Host>::new();

    for scanned_host in scanned_hosts {
        // Check that host has one ip file and one network file
        let mut ip_file: Option<&File> = None;
        let mut network_file: Option<&File> = None;
        let mut windows_tasklist_file: Option<&File> = None;

        for file in scanned_host.files() {
            log::debug!("checking {}", file.path().to_string_lossy());
            match file.file_type() {
                FileType::LinuxIp => ip_file = Some(file),
                FileType::WindowsIp => ip_file = Some(file),
                FileType::LinuxNetstat => network_file = Some(file),
                FileType::WindowsNetstat => network_file = Some(file),
                FileType::LinuxSs => network_file = Some(file),
                FileType::WindowsTasklist => windows_tasklist_file = Some(file),
                FileType::Nmap => {
                    ip_file = Some(file);
                    network_file = Some(file)
                }
                FileType::CsvIp => ip_file = Some(file),
                FileType::CsvNetwork => network_file = Some(file),
            };
        }

        let ip_file = match ip_file {
            Some(n) => n,
            None => {
                bail!(format!(
                    "host {} is missing the ip file",
                    scanned_host.name()
                ))
            }
        };
        let network_file = match network_file {
            Some(n) => n,
            None => {
                bail!(format!(
                    "host {} is missing the network file",
                    scanned_host.name()
                ));
            }
        };

        if let FileType::WindowsNetstat = network_file.file_type() {
            if windows_tasklist_file.is_none() {
                bail!(format!(
                    "host {} is missing the Windows tasklist file",
                    scanned_host.name()
                ));
            }
        };

        // Build the host
        let network_file = network_file;
        let ip_file = ip_file;

        match ip_file.file_type() {
            FileType::LinuxIp => {
                match network_file.file_type() {
                    FileType::LinuxNetstat => {
                        let linux_host_files = LinuxHostFiles::new(
                            scanned_host.name().into(),
                            NetworkOutputFile::Netstat(network_file.path().into()),
                            ip_file.path().into(),
                        );
                        let host: anyhow::Result<Host> = linux_host_files.into();
                        match host {
                            Ok(h) => hosts.push(h),
                            Err(e) => {
                                log::warn!("unable to make host {}: {}", scanned_host.name(), e)
                            }
                        };
                    }
                    FileType::LinuxSs => {
                        let linux_host_files = LinuxHostFiles::new(
                            scanned_host.name().into(),
                            NetworkOutputFile::Ss(network_file.path().into()),
                            ip_file.path().into(),
                        );
                        let host: anyhow::Result<Host> = linux_host_files.into();
                        match host {
                            Ok(h) => hosts.push(h),
                            Err(e) => {
                                log::warn!("unable to make host {}: {}", scanned_host.name(), e)
                            }
                        };
                    }
                    FileType::WindowsNetstat => {
                        bail!("wrong association: Linux ip file with Windows netstat file"
                            .to_string());
                    }
                    _ => continue, // unreachable statement
                }
            }
            FileType::WindowsIp => {
                let windows_tasklist_file = windows_tasklist_file.unwrap();
                let windows_host_files = WindowsHostFiles::new(
                    scanned_host.name().into(),
                    network_file.path().into(),
                    ip_file.path().into(),
                    windows_tasklist_file.path().into(),
                );
                let host: anyhow::Result<Host> = windows_host_files.into();
                match host {
                    Ok(h) => hosts.push(h),
                    Err(e) => bail!(e),
                };
            }
            FileType::Nmap => {
                if let Ok(host) = host::Host::from_nmap_output_file(
                    scanned_host.name(),
                    ip_file.path().to_path_buf(),
                ) {
                    hosts.push(host);
                }
            }
            FileType::CsvIp => {
                if let Ok(host) = host::Host::from_csv_files(
                    scanned_host.name(),
                    network_file.path().to_path_buf(),
                    ip_file.path().to_path_buf(),
                ) {
                    hosts.push(host);
                }
            }
            _ => continue, // unreachable statement
        }
    }
    Ok(hosts)
}
