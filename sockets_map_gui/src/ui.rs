mod app_msgs;
mod files;
mod graph_options;
mod help;
mod server;

use anyhow::bail;
use gtk::{
    glib::clone,
    prelude::FileExt,
    traits::{
        BoxExt, ButtonExt, FileChooserExt, GtkWindowExt, NativeDialogExt, ToggleButtonExt,
        WidgetExt,
    },
    FileChooser, FileFilter,
};
use relm4::{
    adw, factory::FactoryVecDeque, Component, ComponentController, ComponentParts, Controller,
    MessageBroker, RelmContainerExt,
};
use sockets_map::{
    host::Host,
    parsers::directory_scanner::ScannedHost,
    server::{client::Client, message::Message},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tsyncp::{self, broadcast::BincodeSender};

use self::{
    app_msgs::{AppCmdOutput, GraphMsg, ServerMsg},
    files::{FilesOptions, FilesPageWidgets},
    graph_options::{GraphOptions, GraphPageWidgets, DEFAULT_DPI},
    help::HelpWindow,
    server::{
        client::{ClientInfo, ClientLabelMsg},
        ServerPageWidgets,
    },
};

static HELP_WINDOW_BROKER: MessageBroker<help::HelpWindow> = MessageBroker::new();

#[tracker::track]
pub struct AppModel {
    #[tracker::do_not_track]
    /// A temporary file that will receive the generated graph
    image_graph_tempfile: tempfile::NamedTempFile,
    /// The error message to be shown in the info bar
    error_message: Option<String>,
    /// Whether the graph is being generated or not
    generating_graph: bool,
    pub graph_image_path: Option<PathBuf>,
    /// Input files parameters
    #[tracker::do_not_track]
    files_options: FilesOptions,
    /// Server state
    #[tracker::do_not_track]
    pub server_state: ServerState,
    /// Graph parameters as shown in the graph
    #[tracker::do_not_track]
    graph_options: GraphOptions,
    /// Client labels to show the list
    #[tracker::do_not_track]
    clients: FactoryVecDeque<server::client::ClientLabel>,
    #[tracker::do_not_track]
    /// Recording indicator used by the recorder timer
    recording_since: Option<std::time::Instant>,
}

#[derive(Debug)]
pub enum AppMsg {
    Error(Option<String>),
    ServerMsg(ServerMsg),
    GraphMsg(GraphMsg),
}

#[allow(unused)]
pub struct AppWidgets {
    info_bar_msg: gtk::Label,
    info_bar: gtk::InfoBar,
    files_page_widgets: FilesPageWidgets,
    graph_page_widgets: GraphPageWidgets,
    export_graph_button: gtk::Button,
    open_graph_button: gtk::Button,
    server_page_widgets: ServerPageWidgets,
    #[allow(unused)]
    help_window: Controller<HelpWindow>,
}

impl Component for AppModel {
    type CommandOutput = AppCmdOutput;
    type Input = AppMsg;
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        #[cfg(target_os = "windows")]
        set_dark_theme();

        let window = adw::Window::builder()
            .default_width(1000)
            .default_height(600)
            .title("Socket Map")
            .build();
        window.connect_close_request(move |w| {
            w.close();
            std::process::exit(0);
        });

        window
    }

    fn init(
        _init: Self::Init,
        app_window: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let outer_box = gtk::Box::builder()
            .hexpand(true)
            .vexpand(true)
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Header bar
        let title = adw::WindowTitle::new("Sockets map", "");
        let header_bar = adw::HeaderBar::builder().title_widget(&title).build();

        // Flap that has a stack with three pages: graph options, server and files
        let flap = adw::Flap::builder()
            .fold_threshold_policy(adw::FoldThresholdPolicy::Minimum)
            .transition_type(adw::FlapTransitionType::Over)
            .flap_position(gtk::PackType::Start)
            .build();

        // Sidebar button
        let sidebar_button = gtk::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Show or hide sidebar")
            .build();
        sidebar_button.connect_clicked(clone!(@strong sender, @strong flap => move |_| {
            if flap.reveals_flap() {
                flap.set_reveal_flap(false);
            } else {
                flap.set_reveal_flap(true);
            }
        }));
        header_bar.pack_start(&sidebar_button);

        // Sidebar outer box
        let sidebar_content_clamp = adw::Clamp::builder()
            .maximum_size(50)
            .hexpand(false)
            .build();
        let sidebar_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(vec!["background".to_string()])
            .hexpand(false)
            .build();
        sidebar_content_clamp.set_child(Some(&sidebar_content_box));
        flap.set_flap(Some(&sidebar_content_clamp));

        // Sidebar stack
        let sidebar_stack = adw::ViewStack::builder().vexpand(true).build();
        sidebar_content_box.append(&sidebar_stack);

        // Sidebar switcher bar
        let sidebar_switcher = adw::ViewSwitcherBar::builder()
            .reveal(true)
            .stack(&sidebar_stack)
            .build();
        sidebar_content_box.append(&sidebar_switcher);

        // Sidebar graph widgets
        let (graph_options, graph_page_widgets) =
            graph_options::init_sidebar_graph_page_widgets(&sidebar_stack, &flap, sender.clone());

        // Sidebar server widgets
        let (server_page_widgets, clients) =
            server::init_sidebar_server_widgets(&sidebar_stack, sender.clone());

        // Sidebar files widgets
        let files_page_widgets =
            files::init_sidebar_files_widgets(&sidebar_stack, sender.clone(), app_window);

        // File chooser
        let file_chooser = gtk::FileChooserNative::new(
            Some("Export graph"),
            Some(app_window),
            gtk::FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );
        file_chooser.set_select_multiple(false);
        let filter = FileFilter::new();
        filter.add_mime_type("image/svg");
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("text/csv");
        file_chooser.set_filter(&filter);
        file_chooser.connect_response(
            clone!(@strong sender  => move |file_chooser, response_type| {
                if response_type == gtk::ResponseType::Accept {
                    let chooser: FileChooser = file_chooser.to_owned().into();
                    if let Some(file) = chooser.file().and_then(|d| d.path()) {
                        sender.input(AppMsg::GraphMsg(GraphMsg::ExportGraph(file)));
                    }
                }

                file_chooser.hide();
            }),
        );

        // Help button and window
        let help_window = help::HelpWindow::builder()
            .transient_for(app_window)
            .launch_with_broker((), &HELP_WINDOW_BROKER)
            .detach();
        let help_window_sender = help_window.sender();
        let help_button = gtk::Button::builder()
            .icon_name("dialog-question-symbolic")
            .build();
        help_button.connect_clicked(clone!(@strong help_window_sender => move |_| {
            help_window_sender.emit(help::HelpWindowMsg::Show)
        }));
        header_bar.pack_end(&help_button);

        // Export button
        let export_graph_button = gtk::Button::builder()
            .sensitive(false)
            .has_frame(true)
            .build();
        let export_graph_button_content = adw::ButtonContent::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Export graph to file")
            .label("Export")
            .use_underline(true)
            .build();
        export_graph_button.set_child(Some(&export_graph_button_content));
        export_graph_button.connect_clicked(clone!(@strong file_chooser => move |_| {
            file_chooser.show()
        }));
        header_bar.pack_end(&export_graph_button);

        // Open in external viewer button
        let open_graph_button = gtk::Button::builder()
            .sensitive(false)
            .has_frame(true)
            .build();
        let open_graph_button_content = adw::ButtonContent::builder()
            .icon_name("document-open-symbolic")
            .label("Open in viewer")
            .tooltip_text("Open in external viewer")
            .use_underline(true)
            .build();
        open_graph_button.set_child(Some(&open_graph_button_content));
        open_graph_button.connect_clicked(clone!(@strong sender => move |_| {
            sender.input(AppMsg::GraphMsg(GraphMsg::OpenInViewer));
        }));
        header_bar.pack_end(&open_graph_button);

        // Info bar
        let info_bar = gtk::InfoBar::builder()
            .revealed(false)
            .message_type(gtk::MessageType::Error)
            .show_close_button(true)
            .build();
        let info_bar_msg = gtk::Label::builder()
            .hexpand(true)
            .halign(gtk::Align::Center)
            .build();
        info_bar.add_child(&info_bar_msg);
        info_bar.connect_response(clone!(@strong sender => move |info_bar, _response_type| {
            sender.input(AppMsg::Error(None));
            info_bar.set_revealed(false);
        }));

        // Add widgets
        outer_box.append(&header_bar);
        outer_box.append(&info_bar);
        outer_box.append(&flap);
        app_window.container_add(&outer_box);

        ComponentParts {
            model: AppModel {
                image_graph_tempfile: generate_png_temp_file_path(),
                error_message: None,
                generating_graph: false,
                server_state: ServerState {
                    run_token: CancellationToken::new(),
                    clients: Arc::new(RwLock::new(HashMap::new())),
                    is_enabled: false,
                    tx: Arc::new(RwLock::new(None)),
                },
                graph_options,
                graph_image_path: None,
                tracker: 0,
                files_options: FilesOptions::default(),
                clients,
                recording_since: None,
            },
            widgets: AppWidgets {
                info_bar_msg,
                info_bar,
                files_page_widgets,
                graph_page_widgets,
                export_graph_button,
                server_page_widgets,
                open_graph_button,
                help_window,
            },
        }
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.reset();
        self.graph_options.reset();
        self.files_options.reset();
        match message {
            AppMsg::GraphMsg(msg) => self.handle_graph_message(msg, &sender),
            AppMsg::Error(error_msg) => self.set_error_message(error_msg),
            AppMsg::ServerMsg(msg) => self.handle_server_message(msg, &sender),
        }

        // Regenerate graph if options are changed
        if self.graph_options.changed(GraphOptions::track_all())
            && self.get_graph_image_path().is_some()
        {
            sender.input(AppMsg::GraphMsg(GraphMsg::Generating(true)));
        }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.reset();
        self.graph_options.reset();
        self.files_options.reset();
        match message {
            AppCmdOutput::GeneratedGraph(image_path) => {
                if let Some(image_path) = &image_path {
                    log::info!("generated graph at {image_path:?}");
                    sender.input(AppMsg::Error(None));
                } else {
                    log::info!("did not generate graph");
                }
                sender.input(AppMsg::GraphMsg(GraphMsg::Generating(false)));
                sender.input(AppMsg::GraphMsg(GraphMsg::SetImagePath(image_path)));
            }
            AppCmdOutput::SetServerIsEnabled(server_is_enabled) => {
                self.server_state.is_enabled = server_is_enabled;
                if !server_is_enabled {
                    self.clients.guard().clear();
                }
            }
            AppCmdOutput::Error(error_msg) => self.set_error_message(error_msg),
            AppCmdOutput::RecorderTimerTick => {
                if let Some(recording_since) = self.recording_since {
                    // Update label
                    let now = std::time::Instant::now();
                    let interval = now - recording_since;
                    let interval = Duration::new(interval.as_secs(), 0);
                    widgets
                        .server_page_widgets
                        .recorder_timer
                        .set_label(&format!(
                        "<span size=\"small\" foreground=\"grey\"><i>(recording for {})</i></span>",
                        humantime::format_duration(interval)
                    ));

                    // Send next tick
                    sender.oneshot_command(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        AppCmdOutput::RecorderTimerTick
                    });
                } else {
                    // Disabling the button will hide the recording timer
                    widgets
                        .server_page_widgets
                        .clients_record_button
                        .set_child(Some(
                            &widgets.server_page_widgets.client_record_button_content,
                        ));
                    widgets
                        .server_page_widgets
                        .clients_record_button
                        .set_active(false);
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: relm4::ComponentSender<Self>) {
        // Main window view
        if self.changed(Self::error_message()) {
            if let Some(error_msg) = self.get_error_message() {
                widgets.info_bar_msg.set_label(error_msg);
                widgets.info_bar.set_revealed(true);
            } else {
                widgets.info_bar_msg.set_label("");
                widgets.info_bar.set_revealed(false);
            }
        }

        // Graph page view
        if self.changed(Self::generating_graph()) {
            widgets
                .graph_page_widgets
                .generate_button_spinner
                .set_spinning(*self.get_generating_graph());
            widgets
                .graph_page_widgets
                .generate_button_spinner
                .set_visible(*self.get_generating_graph());
        }
        if self.changed(Self::graph_image_path()) {
            if let Some(graph_image_path) = self.get_graph_image_path() {
                widgets
                    .graph_page_widgets
                    .graph_image
                    .set_filename(Some(&graph_image_path));
                widgets
                    .graph_page_widgets
                    .image_view_stack
                    .set_visible_child(&widgets.graph_page_widgets.graph_image);
                widgets.export_graph_button.set_sensitive(true);
                widgets.open_graph_button.set_sensitive(true);
            }
        }

        // Files page view
        if self.files_options.changed(FilesOptions::input_directory()) {
            widgets.files_page_widgets.folder_label.set_markup(
                &self
                    .files_options
                    .input_directory
                    .clone()
                    .and_then(|dir| dir.file_name().map(|s| s.to_string_lossy().to_string()))
                    .map(|dir_name| format!("Selected folder: <b>{dir_name}</b>"))
                    .unwrap_or_else(|| "Select folder to add hosts".to_string()),
            );
            widgets
                .files_page_widgets
                .delete_button
                .set_visible(self.files_options.input_directory.is_some());
        }
        if self.files_options.changed(FilesOptions::scanned_hosts()) {
            if let Some(hosts) = self.files_options.get_scanned_hosts() {
                let mut text = String::from("<b>Parsed hosts:</b>\n\n");
                for host in hosts {
                    text.push_str(&format!("- {}\n", host.name()));
                }
                widgets.files_page_widgets.hosts_text.set_markup(&text);
                widgets.files_page_widgets.separator.set_visible(true);
            } else {
                widgets.files_page_widgets.hosts_text.set_text("");
                widgets.files_page_widgets.separator.set_visible(false);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn set_dark_theme() {
    let display = gtk::gdk::Display::default().expect("unable to get default display");
    let style_manager = adw::StyleManager::for_display(&display);
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
}

impl AppModel {
    fn regenerate_temp_png_file_path(&mut self) {
        let named_temp_file = generate_png_temp_file_path();
        self.image_graph_tempfile = named_temp_file;
    }

    fn handle_server_message(&mut self, msg: ServerMsg, sender: &relm4::ComponentSender<AppModel>) {
        match msg {
            ServerMsg::SetServerIsEnabled(enabled) => {
                self.server_state.is_enabled = enabled;
                if !enabled {
                    self.clients.guard().clear();
                }
            }
            ServerMsg::ClientConnect(client) => {
                self.clients.guard().push_back(client);
            }
            ServerMsg::ClientDisconnect(client) => {
                let client_index = self
                    .clients
                    .guard()
                    .iter()
                    .find(|c| c.info == client)
                    .map(|c| c.index.current_index());
                if let Some(index) = client_index {
                    self.clients.guard().remove(index);
                }
            }
            ServerMsg::ClientUpdate(client) => {
                let client_index = self
                    .clients
                    .guard()
                    .iter()
                    .find(|c| c.info == client)
                    .map(|c| c.index.current_index());
                if let Some(index) = client_index {
                    self.clients.guard().send(index, ClientLabelMsg::GotUpdate);
                }
            }
            ServerMsg::StartRecorder(interval) => {
                let tx_opt = self.server_state.tx.clone();
                self.clients
                    .guard()
                    .broadcast(ClientLabelMsg::Recording(true));
                self.recording_since = Some(std::time::Instant::now());
                sender.oneshot_command(async move {
                    if let Some(tx) = tx_opt.write().await.as_mut() {
                        let (_res, _accept_res) =
                            tx.send(Message::StartRecording(interval)).accepting().await;
                    }

                    // Start the timer
                    AppCmdOutput::RecorderTimerTick
                });
            }
            ServerMsg::StopRecorder => {
                let tx_opt = self.server_state.tx.clone();
                self.clients
                    .guard()
                    .broadcast(ClientLabelMsg::Recording(false));
                self.recording_since = None;
                sender.oneshot_command(async move {
                    if let Some(tx) = tx_opt.write().await.as_mut() {
                        let (_res, _accept_res) = tx.send(Message::StopRecording).accepting().await;
                    }
                    AppCmdOutput::Error(None)
                })
            }
            ServerMsg::EnableServer(server_options) => {
                if let Some(server_options) = server_options {
                    let clients = self.server_state.clients.clone();
                    self.server_state.run_token = CancellationToken::new();
                    let token = self.server_state.run_token.clone();
                    let tx_opt = self.server_state.tx.clone();
                    sender.input(AppMsg::ServerMsg(ServerMsg::SetServerIsEnabled(true)));
                    sender.oneshot_command(clone!(@strong sender => async move {
                        log::info!("starting server");
                        match sockets_map::server::listen(
                            format!(
                                "{}:{}",
                                server_options.listen_addr, server_options.listen_port
                            ),
                            clients,
                            token,
                            |socket_addr| {
                                log::info!("connection from peer {socket_addr:?}");
                            },
                            {
                                let sender = sender.clone();
                                move |client: &Client| {
                                    log::info!(
                                        "client registration for {:?}",
                                        &client.pretty_name.as_ref().unwrap_or(&client.hostname)
                                    );
                                    sender.input(AppMsg::ServerMsg(ServerMsg::ClientConnect(
                                        ClientInfo {
                                            hostname: client.hostname.clone(),
                                            pretty_name: client.pretty_name.clone(),
                                            ips: client.ips.clone()
                                    })));
                                }
                            },
                            {
                                let sender = sender.clone();
                                move |client: &Client| {
                                    log::info!(
                                        "client update ({:?})",
                                        &client.pretty_name.as_ref().unwrap_or(&client.hostname)
                                    );
                                    sender.input(AppMsg::ServerMsg(ServerMsg::ClientUpdate(
                                        ClientInfo {
                                            hostname: client.hostname.clone(),
                                            pretty_name: client.pretty_name.clone(),
                                            ips: client.ips.clone()
                                        }
                                    )))
                                }
                            },
                            {
                                let sender = sender.clone();
                                move |client: &Client| {
                                    log::info!(
                                        "client exit ({:?})",
                                        &client.pretty_name.as_ref().unwrap_or(&client.hostname)
                                    );
                                    sender.input(AppMsg::ServerMsg(ServerMsg::ClientDisconnect(
                                        ClientInfo {
                                            hostname: client.hostname.clone(),
                                            pretty_name: client.pretty_name.clone(),
                                            ips: client.ips.clone()
                                    })));
                                }
                            },
                        )
                        .await {
                            Ok(tx) => {
                                *tx_opt.write().await = Some(tx);
                                AppCmdOutput::Error(None)
                            },
                            Err(e) => AppCmdOutput::Error(Some(e.to_string())),
                        }
                    }));
                } else if self.server_state.is_enabled {
                    log::info!("stopping server");
                    // If recorder was running, stop it
                    sender.input(AppMsg::ServerMsg(ServerMsg::StopRecorder));

                    // Stop the server
                    let token = self.server_state.run_token.clone();
                    let tx_opt = self.server_state.tx.clone();
                    sender.oneshot_command(async move {
                        // Disconnect clients
                        // Taking the `tx_opt` value here drops it at then end and closes the listening socket
                        if let Some(mut tx) = tx_opt.write().await.take() {
                            let (_res, _accept_res) = tx.send(Message::Exit).accepting().await;
                        }

                        // Shutdown server
                        token.cancel();
                        AppCmdOutput::SetServerIsEnabled(false)
                    });
                    self.server_state.is_enabled = false;
                }
            }
            ServerMsg::SendUpdateRequest => {
                let tx_opt = self.server_state.tx.clone();
                sender.oneshot_command(async move {
                    if let Some(tx) = tx_opt.write().await.as_mut() {
                        let (_res, _accept_res) = tx.send(Message::UpdateRequest).accepting().await;
                    }
                    AppCmdOutput::Error(None)
                })
            }
        }
    }

    fn handle_graph_message(&mut self, msg: GraphMsg, sender: &relm4::ComponentSender<AppModel>) {
        match msg {
            GraphMsg::GenerateGraph(graph_options) => {
                self.regenerate_temp_png_file_path();

                let scanned_hosts = self.files_options.scanned_hosts.clone();
                let image_graph_tempfile_path = self.image_graph_tempfile.path().to_path_buf();
                let input_sender = sender.input_sender().clone();
                let clients = self.server_state.clients.clone();
                let tx_opt = self.server_state.tx.clone();
                sender.oneshot_command(async move {
                    match generate_graph(
                        scanned_hosts,
                        clients,
                        tx_opt,
                        &graph_options,
                        &image_graph_tempfile_path,
                        None,
                    )
                    .await
                    {
                        Ok(_) => AppCmdOutput::GeneratedGraph(Some(image_graph_tempfile_path)),
                        Err(e) => {
                            input_sender.emit(AppMsg::Error(Some(e.to_string())));
                            AppCmdOutput::GeneratedGraph(None)
                        }
                    }
                });
            }
            GraphMsg::Generating(generating) => {
                self.set_generating_graph(generating);
                if generating {
                    // Generate the graph
                    sender.input(AppMsg::GraphMsg(GraphMsg::GenerateGraph(
                        self.graph_options.clone(),
                    )));
                }
            }

            GraphMsg::SetHideLoopbackConnections(value) => {
                self.graph_options.set_hide_loopback_connections(value)
            }
            GraphMsg::SetVerticalGraph(value) => self.graph_options.set_vertical_graph(value),
            GraphMsg::SetTransparentBackground(value) => {
                self.graph_options.set_transparent_background(value)
            }
            GraphMsg::SetHideLegend(value) => self.graph_options.set_hide_legend(value),
            GraphMsg::SetHideAgents(value) => self.graph_options.set_hide_agents(value),
            GraphMsg::SetImagePath(image_path) => self.set_graph_image_path(image_path),
            GraphMsg::SetInputDir(dir) => {
                self.files_options.set_input_directory(dir.clone());
                if let Some(dir) = dir {
                    self.files_options.set_scanned_hosts(Some(
                        sockets_map::parsers::directory_scanner::scan_dir(&dir),
                    ));
                } else {
                    self.files_options.set_scanned_hosts(None);
                }
            }
            GraphMsg::SetFileExtension(file_extension) => {
                self.graph_options.set_file_extension(file_extension);
            }
            GraphMsg::ExportGraph(path) => {
                if let Err(msg) = std::fs::copy(
                    self.image_graph_tempfile.path(),
                    path.with_extension(self.graph_options.file_extension.clone()),
                ) {
                    self.set_error_message(Some(msg.to_string()));
                };
            }
            GraphMsg::TrySetOutputDPI(dpi_str) => match dpi_str.parse::<f64>() {
                Ok(dpi) => {
                    self.graph_options.dpi = dpi;
                }
                Err(e) => {
                    if !dpi_str.is_empty() {
                        sender.input(AppMsg::Error(Some(e.to_string())));
                    }
                    self.graph_options.dpi = DEFAULT_DPI;
                }
            },
            GraphMsg::SetLayoutEngine(layout_engine) => {
                self.graph_options.set_layout_engine(layout_engine)
            }
            GraphMsg::OpenInViewer => {
                if let Some(p) = &self.graph_image_path {
                    if let Err(e) = open::that(p) {
                        log::error!("unable to open {p:?} in external viewer: {e}");
                    }
                }
            }
        }
    }
}

fn generate_png_temp_file_path() -> tempfile::NamedTempFile {
    let named_temp_file = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("unable to create temporary file");
    named_temp_file
}

async fn generate_graph(
    scanned_hosts: Option<Vec<ScannedHost>>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    tx_opt: Arc<RwLock<Option<BincodeSender<Message>>>>,
    graph_options: &GraphOptions,
    output_file: &Path,
    dump_dot_code: Option<&PathBuf>,
) -> anyhow::Result<()> {
    // If the server is running and does not have got any update yet, send a request to clients
    if let Some(tx) = tx_opt.write().await.as_mut() {
        if !clients
            .read()
            .await
            .iter()
            .any(|(_name, client)| !client.updates().is_empty())
        {
            log::info!("sending update request to clients");
            let (_res, _accept_res) = tx.send(Message::UpdateRequest).accepting().await;
            log::debug!("peers when sending: {:?}", tx.peer_addrs());

            // Wait for all clients to send their update, with a timeout
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut number_of_remaining_intervals = 20;
            let mut still_missing_all_updates = true;
            while number_of_remaining_intervals > 0 {
                number_of_remaining_intervals -= 1;
                interval.tick().await;
                if clients
                    .read()
                    .await
                    .iter()
                    .any(|(_name, client)| client.updates().is_empty())
                {
                    continue;
                } else {
                    number_of_remaining_intervals = 0;
                    still_missing_all_updates = false;
                }
            }

            if still_missing_all_updates {
                log::warn!("did not get an update from all clients"); // TODO: show in GUI
            }
        }
    }

    let clients = clients.read().await;

    // Scanned hosts
    let mut hosts = scanned_hosts
        .and_then(|scanned_hosts| {
            sockets_map::parsers::directory_scanner::build_hosts(&scanned_hosts).ok()
        })
        .unwrap_or_default();

    // Client hosts
    let client_hosts: Vec<Host> = clients
        .iter()
        .filter_map(|(_name, client)| client.updates().last().map(|update| update.host.clone()))
        .collect();
    hosts.extend(client_hosts);
    if hosts.is_empty() {
        bail!("No hosts to generate graph from");
    }

    // Exclude some processes from the analysis
    if *graph_options.get_hide_agents() {
        let excluded_processes = ["sockets_map"];
        for host in hosts.iter_mut() {
            host.exclude_processes(&excluded_processes);
        }
    }

    // Generate connections
    let connections = sockets_map::connections_model::build_connections_list(
        &hosts,
        graph_options.hide_loopback_connections,
    );

    // Generate the Dot graph
    let graph = sockets_map::graphs::create_graph(
        &connections,
        graph_options.transparent_background,
        graph_options.hide_legend,
        graph_options.dpi,
        Some(&graph_options.layout_engine),
    )?;

    // Run Graphviz command to generate the graph
    sockets_map::graphviz::run_graphviz(
        graph.to_string(),
        output_file,
        graph_options.file_extension.clone(),
        dump_dot_code,
        graph_options.vertical_graph,
        Some(&graph_options.layout_engine),
    )?;

    Ok(())
}

pub struct ServerState {
    /// Whether the GUI should ask the server to start or stop
    run_token: CancellationToken,
    pub clients: Arc<RwLock<HashMap<String, Client>>>,
    /// Whether the server is running or not
    pub is_enabled: bool,
    /// Channel sender
    pub tx: Arc<RwLock<Option<BincodeSender<Message>>>>,
}
