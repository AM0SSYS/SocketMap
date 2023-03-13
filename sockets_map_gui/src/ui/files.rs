//! File input page widgets

mod cheatsheet;

use std::path::PathBuf;

use gtk::{
    glib::clone,
    prelude::FileExt,
    traits::{BoxExt, ButtonExt, FileChooserExt, NativeDialogExt},
    FileChooser, FileFilter,
};
use relm4::{adw, ComponentController, Controller};
use relm4::{Component, MessageBroker, RelmWidgetExt};
use sockets_map::parsers::directory_scanner::ScannedHost;

use super::AppModel;
use super::{app_msgs::GraphMsg, AppMsg};

use relm4::ComponentSender;

static CHEATSHEET_WINDOW_BROKER: MessageBroker<cheatsheet::CheatsheetWindow> = MessageBroker::new();

#[tracker::track]
#[derive(Default)]
pub(crate) struct FilesOptions {
    /// The directory from which the app will parse static hosts
    pub input_directory: Option<PathBuf>,
    #[tracker::no_eq]
    /// The parsed static hosts
    pub scanned_hosts: Option<Vec<ScannedHost>>,
}

pub(crate) struct FilesPageWidgets {
    pub hosts_text: gtk::Label,
    pub separator: gtk::Separator,
    pub folder_label: gtk::Label,
    pub delete_button: gtk::Button,
    #[allow(unused)]
    pub cheatsheet_window: Controller<cheatsheet::CheatsheetWindow>,
}

/// Generate the input files controls widgets for the sidebar
pub(crate) fn init_sidebar_files_widgets(
    sidebar_stack: &adw::ViewStack,
    sender: ComponentSender<AppModel>,
    main_window: &adw::Window,
) -> FilesPageWidgets {
    let scrollable_file_box = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scrollable_file_box.set_margin_all(20);
    let files_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .width_request(300)
        .build();
    scrollable_file_box.set_child(Some(&files_box));

    // File chooser
    let file_chooser = gtk::FileChooserNative::new(
        Some("Add input directory"),
        Some(main_window),
        gtk::FileChooserAction::SelectFolder,
        Some("Open"),
        Some("Cancel"),
    );
    file_chooser.set_select_multiple(false);
    let filter = FileFilter::new();
    filter.add_mime_type("inode/directory");
    file_chooser.set_filter(&filter);
    file_chooser.connect_response(
        clone!(@strong sender  => move |file_chooser, response_type| {
            if response_type == gtk::ResponseType::Accept {
                let chooser: FileChooser = file_chooser.to_owned().into();
                if let Some(directory) = chooser.file().and_then(|d| d.path()) {
                    sender.input(AppMsg::GraphMsg(GraphMsg::SetInputDir(Some(directory))));
                }
            }

            file_chooser.hide();
        }),
    );

    // Cheatsheet window
    let cheatsheet_window = cheatsheet::CheatsheetWindow::builder()
        .transient_for(main_window)
        .launch_with_broker((), &CHEATSHEET_WINDOW_BROKER)
        .detach();
    let cheatsheet_window_sender = cheatsheet_window.sender();

    // Open buttons
    let buttons_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .hexpand(false)
        .halign(gtk::Align::Center)
        .build();
    let add_files_button = gtk::Button::new();
    let add_files_button_content = adw::ButtonContent::builder()
        .icon_name("document-open-symbolic")
        .label("Open")
        .build();
    add_files_button.set_child(Some(&add_files_button_content));
    add_files_button.connect_clicked(
        clone!(@strong sender, @strong file_chooser, @strong main_window => move |_| {
            file_chooser.show();
            file_chooser.set_transient_for(Some(&main_window));
        }),
    );
    buttons_box.append(&add_files_button);

    // Cheatsheets buttons
    let cheatsheet_button = gtk::Button::new();
    let cheatsheet_button_content = adw::ButtonContent::builder()
        .icon_name("dialog-question-symbolic")
        .label("Cheatsheet")
        .build();
    cheatsheet_button.set_child(Some(&cheatsheet_button_content));
    cheatsheet_button.connect_clicked(clone!(@strong cheatsheet_window_sender => move |_| {
        cheatsheet_window_sender.emit(cheatsheet::CheatsheetWindowMsg::Show)
    }));
    buttons_box.append(&cheatsheet_button);

    // Selected folder display
    let selected_folder_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk::Align::Center)
        .build();
    let delete_button = gtk::Button::builder()
        .icon_name("delete")
        .css_classes(vec!["destructive-action".to_string()])
        .visible(false)
        .build();
    delete_button.connect_clicked(clone!(@strong sender => move |_| {
        sender.input(AppMsg::GraphMsg(GraphMsg::SetInputDir(None)))
    }));
    let folder_label = gtk::Label::builder()
        .label("Select folder to add hosts")
        .max_width_chars(20)
        .wrap(true)
        .build();
    selected_folder_box.append(&delete_button);
    selected_folder_box.append(&folder_label);

    // Label to show parses hosts
    let separator = gtk::Separator::builder()
        .orientation(gtk::Orientation::Horizontal)
        .visible(false)
        .build();
    let hosts_text = gtk::Label::builder().use_markup(true).build();

    files_box.append(&buttons_box);
    files_box.append(&selected_folder_box);
    files_box.append(&separator);
    files_box.append(&hosts_text);

    // Add to the sidebar view stack
    sidebar_stack.add(&scrollable_file_box);
    sidebar_stack
        .page(&scrollable_file_box)
        .set_icon_name(Some("document-open-symbolic"));
    sidebar_stack
        .page(&scrollable_file_box)
        .set_title(Some("Files"));

    FilesPageWidgets {
        hosts_text,
        separator,
        folder_label,
        delete_button,
        cheatsheet_window,
    }
}
