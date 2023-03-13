use sockets_map::graphviz::LayoutEngine;
use std::path::PathBuf;

use super::{graph_options::GraphOptions, server::client::ClientInfo};

#[derive(Debug)]
pub struct ServerOption {
    pub listen_addr: String,
    pub listen_port: String,
}

#[derive(Debug)]
pub enum ServerMsg {
    EnableServer(Option<ServerOption>),
    /// One time update request
    SendUpdateRequest,
    /// Start the recorder by sending a recorder request to agents
    StartRecorder(f64),
    /// Stop the recorder and collect data
    StopRecorder,
    SetServerIsEnabled(bool),
    ClientConnect(ClientInfo),
    ClientDisconnect(ClientInfo),
    ClientUpdate(ClientInfo),
}

#[derive(Debug)]
pub enum GraphMsg {
    GenerateGraph(GraphOptions),
    Generating(bool),
    /// If `Some`, server is enabled with the options,
    /// otherwise it is disabled.
    SetHideLoopbackConnections(bool),
    SetVerticalGraph(bool),
    SetTransparentBackground(bool),
    SetHideLegend(bool),
    SetHideAgents(bool),
    SetImagePath(Option<PathBuf>),
    SetFileExtension(String),
    TrySetOutputDPI(String),
    SetLayoutEngine(LayoutEngine),
    /// Sent by the files stack page
    SetInputDir(Option<PathBuf>),
    ExportGraph(PathBuf),
    OpenInViewer,
}

#[derive(Debug)]
pub enum AppCmdOutput {
    GeneratedGraph(Option<PathBuf>),
    SetServerIsEnabled(bool),
    Error(Option<String>),
    RecorderTimerTick,
}
