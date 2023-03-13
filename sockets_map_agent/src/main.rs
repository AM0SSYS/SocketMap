use anyhow::{bail, Context};
use clap::Parser;
use local_ip_address::list_afinet_netifas;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;

mod args;

use sockets_map::server::{
    client::Update,
    message::{self, Message},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize logger
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("unable to init termlogger");

    // Arguments
    let args = args::Args::parse();

    // Check admin
    if !args.no_root && !collect::ensure_privileged() {
        #[cfg(target_os = "linux")]
        bail!("Must run as root");
        #[cfg(target_os = "windows")]
        bail!("Must run as administrator");
    }

    // Set the locale
    std::env::set_var("LC_ALL", "C");

    // Get local IP addresses
    let local_ips: Vec<IpAddr> = list_afinet_netifas()
        .with_context(|| "unable to retrieve list of local IP addresses: {}")?
        .iter()
        .map(|(_ifname, addr)| *addr)
        .collect();

    // Start client loop
    if let Err(e) = register_and_start_client(args.address, args.pretty_name, local_ips).await {
        log::error!("{e}");
    }

    Ok(())
}

async fn register_and_start_client(
    server_addr: SocketAddr,
    pretty_name: Option<String>,
    ip_addresses: Vec<IpAddr>,
) -> anyhow::Result<()> {
    // Get hostname
    let hostname = hostname::get()?;

    let channel: tsyncp::channel::BincodeChannel<Message> =
        tsyncp::channel::channel_to(server_addr)
            .set_tcp_nodelay(true)
            .await?;
    let (mut rx, tx) = channel.split();
    let tx = Arc::new(RwLock::new(tx));
    let register_message = message::Register::new(
        hostname.to_string_lossy().to_string(),
        pretty_name.clone(),
        ip_addresses.clone(),
    );

    // Create interrupt handler
    let ctrl_c_tx = tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        ctrl_c_tx.write().await.send(Message::Exit).await.unwrap();
        std::process::exit(0);
    });

    // Send registration message
    log::info!("sending registration message");
    let message = Message::Register(register_message);
    tx.write()
        .await
        .send(message)
        .await
        .with_context(|| "unable to send registration message")?;

    // Recorder variables used in tokio jobs
    let recording = Arc::new(RwLock::new(false));
    let host_updates: Arc<RwLock<Vec<Update>>> = Arc::new(RwLock::new(Vec::new()));

    // Listen for instructions
    while let Some(Ok(msg)) = rx.recv().await {
        match msg {
            Message::UpdateRequest => {
                log::info!("sending update");
                let update =
                    collect::generate_one_time_update(&pretty_name, &hostname, &ip_addresses)?;
                let message = Message::Update(update);
                if let Err(e) = tx.write().await.send(message).await {
                    log::error!("failure while sending update: {e}");
                }
            }
            Message::StartRecording(interval) => {
                log::info!("starting recorder with interval of {interval}s");
                *recording.write().await = true;
                host_updates.write().await.clear();
                let host_updates = host_updates.clone();
                let recording = recording.clone();
                let tx = tx.clone();
                let hostname = hostname.clone();
                let ip_addresses = ip_addresses.clone();
                let pretty_name = pretty_name.clone();
                tokio::spawn(async move {
                    // While recording, make updates and wait for the right interval in between
                    while *recording.read().await {
                        if let Ok(update) = collect::generate_one_time_update(
                            &pretty_name,
                            &hostname,
                            &ip_addresses,
                        ) {
                            host_updates.write().await.push(update);
                        }
                        log::info!("captured socket info, waiting for next update");
                        tokio::time::sleep(Duration::from_secs_f64(interval)).await;
                    }

                    // When stopped, send aggregate update
                    match generate_aggregate_update(&host_updates.read().await) {
                        Ok(update) => {
                            let message = Message::Update(update);
                            let mut tx = tx.write().await;
                            if let Err(e) = tx.send(message.clone()).await {
                                log::error!("failure while sending update: {e}");
                            }
                        }
                        // TODO: relay agent errors to the server by making `Update` an enum
                        Err(e) => log::error!("unable to create host object from capture: {e}"),
                    }

                    // Clear updates
                    host_updates.write().await.clear();
                });
            }
            Message::StopRecording => {
                log::info!("stopping recorder and sending aggregate update");
                *recording.write().await = false;
            }
            Message::Exit => {
                log::info!("exiting");
                std::process::exit(0);
            }
            _ => (),
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
mod collect {
    use sockets_map::{
        host::Host, parsers::linux::LinuxHostRawData, server::client::HostData::LinuxHostData,
        server::client::Update,
    };
    use std::{net::IpAddr, process::Command};

    pub type NetworkOutput = sockets_map::parsers::linux::NetworkOutput;

    /// pub Generate an update
    pub fn generate_one_time_update(
        pretty_name: &Option<String>,
        hostname: &std::ffi::OsString,
        ip_addresses: &[IpAddr],
    ) -> Result<Update, anyhow::Error> {
        let linux_host_data = get_host_data(
            pretty_name,
            hostname.to_string_lossy().to_string(),
            ip_addresses,
        )?;
        let host: anyhow::Result<Host> = LinuxHostData(linux_host_data).into();
        let update = Update::new(host?);
        Ok(update)
    }

    /// Retrieve sockets information from the host
    pub fn get_host_data(
        pretty_name: &Option<String>,
        hostname: String,
        ip_addresses: &[IpAddr],
    ) -> Result<LinuxHostRawData, anyhow::Error> {
        let host_data = LinuxHostRawData::new(
            pretty_name.clone().unwrap_or(hostname),
            get_host_sockets_info()?,
            ip_addresses.to_vec(),
        );
        Ok(host_data)
    }

    /// Retrieve network sockets information from the host
    /// First try ss, then netstat
    pub fn get_host_sockets_info() -> anyhow::Result<NetworkOutput> {
        if let Ok(output) = exec_ss() {
            Ok(NetworkOutput::Ss(output))
        } else {
            Ok(NetworkOutput::Netstat(exec_netstat()?))
        }
    }

    fn exec_ss() -> anyhow::Result<String> {
        let output = Command::new("ss").arg("-apn").output()?;
        let output_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_str.to_string())
    }

    fn exec_netstat() -> anyhow::Result<String> {
        // netstat -Wltpn; netstat -Wtpn
        let output1 = Command::new("netstat").arg("-Wltpn").output()?;
        let output2 = Command::new("netstat").arg("-Wtpn").output()?;
        let output_str1 = std::str::from_utf8(&output1.stdout)?;
        let output_str2 = std::str::from_utf8(&output2.stdout)?;

        Ok(format!("{output_str1}\n{output_str2}"))
    }

    pub fn ensure_privileged() -> bool {
        is_sudo::RunningAs::Root == is_sudo::check()
    }
}

#[cfg(target_os = "windows")]
mod collect {
    use sockets_map::{
        host::Host,
        parsers::windows::WindowsHostRawData,
        server::client::{HostData::WindowsHostData, Update},
    };
    use std::{net::IpAddr, process::Command};

    /// Generate an update
    pub fn generate_one_time_update(
        pretty_name: &Option<String>,
        hostname: &std::ffi::OsString,
        ip_addresses: &[IpAddr],
    ) -> Result<Update, anyhow::Error> {
        let linux_host_data = get_host_data(
            pretty_name,
            hostname.to_string_lossy().to_string(),
            ip_addresses,
        )?;
        let host: anyhow::Result<Host> = WindowsHostData(linux_host_data).into();
        let update = Update::new(host?);
        Ok(update)
    }

    /// Retrieve sockets information from the host
    pub fn get_host_data(
        pretty_name: &Option<String>,
        hostname: String,
        ip_addresses: &[IpAddr],
    ) -> Result<WindowsHostRawData, anyhow::Error> {
        let host_data = WindowsHostRawData::new(
            pretty_name.clone().unwrap_or(hostname),
            get_host_sockets_info()?,
            exec_tasklist()?,
            ip_addresses.to_vec(),
        );
        Ok(host_data)
    }

    /// Retrieve network sockets information from the host
    pub fn get_host_sockets_info() -> anyhow::Result<String> {
        let output = Command::new("netstat").arg("-ano").output()?;
        let output_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_str.to_string())
    }

    fn exec_tasklist() -> anyhow::Result<String> {
        let output = Command::new("tasklist").arg("/FO").arg("CSV").output()?;
        let output_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_str.to_string())
    }

    // TODO: need to fix an build issue with the [is_sudo](https://github.com/spa5k/is_sudo) crate
    pub fn ensure_privileged() -> bool {
        true
    }
}

/// This is the equivalent of joining the output of the commands
pub fn generate_aggregate_update(updates: &[Update]) -> Result<Update, anyhow::Error> {
    if let Some(first_update) = updates.first() {
        let mut aggregated_host = first_update.host.clone();
        for host_update in updates.iter().map(|u| u.host.clone()) {
            // Aggregate connections
            aggregated_host
                .connections_mut()
                .extend(host_update.connections().to_vec());
            aggregated_host.connections_mut().sort();
            aggregated_host.connections_mut().dedup();

            // Aggregate listening sockets
            aggregated_host
                .listening_sockets_mut()
                .extend(host_update.listening_sockets().to_vec());
            aggregated_host.listening_sockets_mut().sort();
            aggregated_host.listening_sockets_mut().dedup();
        }

        let update = Update::new(aggregated_host);
        return Ok(update);
    }

    Err(anyhow::Error::msg("no updates were made"))
}
