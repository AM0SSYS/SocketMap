//! Server page widgets

pub mod client;

use gtk::{
    glib::clone,
    prelude::ObjectExt,
    traits::{BoxExt, ButtonExt, EditableExt, ToggleButtonExt, WidgetExt},
};
use relm4::{adw, factory::FactoryVecDeque, ComponentSender, RelmWidgetExt};

use self::client::ClientLabel;

use super::{app_msgs::ServerMsg, app_msgs::ServerOption, AppModel, AppMsg};

#[derive(Debug)]
pub(crate) struct ServerPageWidgets {
    pub recorder_timer: gtk::Label,
    pub clients_record_button: gtk::ToggleButton,
    pub client_record_button_content: adw::ButtonContent,
}

/// Generate the server controls widgets for the sidebar
pub(crate) fn init_sidebar_server_widgets(
    sidebar_stack: &adw::ViewStack,
    sender: ComponentSender<AppModel>,
) -> (ServerPageWidgets, FactoryVecDeque<ClientLabel>) {
    let page_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(5)
        .halign(gtk::Align::Start)
        .build();
    page_box.set_margin_all(10);
    let clamp = adw::Clamp::builder().maximum_size(300).build();

    // Server options
    let server_address_label = gtk::Label::builder()
        .label("Server listen address")
        .hexpand(true)
        .halign(gtk::Align::Start)
        .justify(gtk::Justification::Left)
        .build();
    let server_address = gtk::Entry::builder()
        .tooltip_text("The address the server will listen on")
        .text("0.0.0.0")
        .build();
    let server_port_label = gtk::Label::builder()
        .label("Server listen port")
        .hexpand(true)
        .halign(gtk::Align::Start)
        .justify(gtk::Justification::Left)
        .build();
    let server_port = gtk::Entry::builder()
        .tooltip_text("The TCP port the server will listen on")
        .text("6840")
        .build();

    // Start and stop button
    let server_button_start_content = adw::ButtonContent::builder()
        .icon_name("media-playback-start-symbolic")
        .label("Start server")
        .build();
    let server_button_stop_content = adw::ButtonContent::builder()
        .icon_name("media-playback-stop-symbolic")
        .label("Stop server")
        .build();
    let server_button = gtk::ToggleButton::builder()
        .child(&server_button_start_content)
        .css_classes(vec!["suggested-action".to_string()])
        .build();
    server_button.connect_clicked(
        clone!(@strong sender, @strong server_address, @strong server_port => move |button| {
            if button.is_active() {
                button.set_child(Some(&server_button_stop_content));
                button.set_css_classes(&["destructive-action"]);
                sender.input(AppMsg::ServerMsg(ServerMsg::EnableServer(Some(
                    ServerOption {
                        listen_addr: server_address.text().to_string(),
                        listen_port: server_port.text().to_string(),
                    },
                ))));
            } else {
                button.set_child(Some(&server_button_start_content));
                button.set_css_classes(&["suggested-action"]);
                sender.input(AppMsg::ServerMsg(ServerMsg::EnableServer(None)));
            }
        }),
    );

    // Clients list
    let clients_label_button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .hexpand(true)
        .build();
    let clients_list_label_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .hexpand(true)
        .build();
    let clients_list_label = gtk::Label::builder()
        .label("<b>Active clients</b>")
        .use_markup(true)
        .halign(gtk::Align::Start)
        .build();
    clients_list_label_box.append(&clients_list_label);

    // Update button
    let client_update_button_content = adw::ButtonContent::builder()
        .icon_name("view-refresh-symbolic")
        .label("Update")
        .build();
    let clients_update_button = gtk::Button::builder()
        .tooltip_text("Send a one time update request to clients, to make a graph of their connection at one instant")
        .sensitive(false)
        .child(&client_update_button_content)
        .halign(gtk::Align::End)
        .build();
    clients_update_button.connect_clicked(clone!(@strong sender => move |_| {
        sender.input(AppMsg::ServerMsg(ServerMsg::SendUpdateRequest))
    }));

    // Recorder internal
    let recorder_interval_entry = gtk::Entry::builder()
        .text("1.0")
        .tooltip_text("The interval, in seconds, between updates in Recorder mode")
        .build();

    // Record button
    let client_record_button_content = adw::ButtonContent::builder()
        .icon_name("media-record-symbolic")
        .label("Record")
        .build();
    let client_recording_button_content = adw::ButtonContent::builder()
        .icon_name("media-playback-stop-symbolic")
        .label("Recording")
        .build();
    let clients_record_button = gtk::ToggleButton::builder()
        .tooltip_text("Send update requests to clients until pressed again")
        .tooltip_text("Make regular updates and aggregate them to make a graph of all connections over a period of time")
        .child(&client_record_button_content)
        .halign(gtk::Align::End)
        .sensitive(false)
        .build();
    clients_record_button.connect_clicked(clone!(@strong sender,
        @strong recorder_interval_entry,
        @strong client_record_button_content,
        @strong client_recording_button_content => move |b| {
            if !b.is_active() {
                b.set_child(Some(&client_record_button_content));
                log::info!("stopping recorders");
                sender.input(AppMsg::ServerMsg(ServerMsg::StopRecorder))
            } else if let Ok(i) = recorder_interval_entry.text().parse() {
                if i < 0.1 {
                    b.set_active(false);
                    sender.input(AppMsg::Error(Some("Interval too low (must be >= 0.1s)".into())))
                } else {
                    log::info!("starting recorders");
                    b.set_child(Some(&client_recording_button_content));
                    sender.input(AppMsg::ServerMsg(ServerMsg::StartRecorder(i)))
                }
            } else {
                b.set_active(false);
                sender.input(AppMsg::Error(Some("Invalid recorder update interval (seconds)".into())))
            }
    }));

    // Recorder timer
    let recorder_timer = gtk::Label::builder()
        .use_markup(true)
        .visible(false)
        .hexpand(true)
        .halign(gtk::Align::End)
        .build();
    clients_list_label_box.append(&recorder_timer);

    clients_record_button
        .bind_property("active", &recorder_timer, "visible")
        .build();
    server_button
        .bind_property("active", &clients_record_button, "sensitive")
        .build();
    server_button
        .bind_property("active", &clients_update_button, "sensitive")
        .build();

    // Add buttons and entry
    clients_label_button_box.append(&clients_update_button);
    clients_label_button_box.append(&clients_record_button);
    clients_label_button_box.append(&recorder_interval_entry);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    let clients_box = gtk::ListBox::builder()
        .halign(gtk::Align::Start)
        .hexpand(true)
        .hexpand(true)
        .selection_mode(gtk::SelectionMode::None)
        .build();
    clients_box.set_hexpand(true);
    let clients = FactoryVecDeque::new(clients_box, sender.input_sender());

    // Add to box
    page_box.append(&server_button);
    page_box.append(&server_address_label);
    page_box.append(&server_address);
    page_box.append(&server_port_label);
    page_box.append(&server_port);
    page_box.append(&separator);
    page_box.append(&clients_list_label_box);
    page_box.append(&clients_label_button_box);
    page_box.append(clients.widget());
    clamp.set_child(Some(&page_box));

    sidebar_stack.add(&clamp);
    sidebar_stack
        .page(&clamp)
        .set_icon_name(Some("network-workgroup-symbolic"));
    sidebar_stack.page(&clamp).set_title(Some("Server"));

    let widgets = ServerPageWidgets {
        recorder_timer,
        clients_record_button,
        client_record_button_content,
    };
    (widgets, clients)
}
