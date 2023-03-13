//! Graph options page widgets

use std::str::FromStr;

use super::AppModel;
use super::{app_msgs::GraphMsg, AppMsg};

use gtk::{
    glib::clone,
    traits::{BoxExt, ButtonExt, CheckButtonExt, EditableExt, WidgetExt},
};
use relm4::{adw, ComponentSender, RelmWidgetExt};
use sockets_map::graphviz::LayoutEngine;

const SUPPORTED_FORMATS: [&str; 4] = ["png", "jpeg", "svg", "bmp"];
pub const DEFAULT_DPI: f64 = 96.0;

#[derive(Debug)]
pub(crate) struct GraphPageWidgets {
    pub generate_button_spinner: gtk::Spinner,
    pub image_view_stack: gtk::Stack,
    pub graph_image: gtk::Picture,
}

#[tracker::track]
#[derive(Debug, Clone)]
pub struct GraphOptions {
    pub hide_loopback_connections: bool,
    pub vertical_graph: bool,
    pub transparent_background: bool,
    pub hide_legend: bool,
    pub file_extension: String,
    pub dpi: f64,
    pub hide_agents: bool,
    pub layout_engine: LayoutEngine,
}

impl GraphOptions {
    pub fn new() -> Self {
        Self {
            hide_loopback_connections: false,
            vertical_graph: false,
            transparent_background: false,
            hide_legend: false,
            file_extension: "png".into(),
            tracker: 0,
            dpi: DEFAULT_DPI,
            hide_agents: true,
            layout_engine: LayoutEngine::Dot,
        }
    }
}

/// Generate the graph controls widgets for the sidebar
pub(crate) fn init_sidebar_graph_page_widgets(
    sidebar_stack: &adw::ViewStack,
    flap: &adw::Flap,
    sender: ComponentSender<AppModel>,
) -> (GraphOptions, GraphPageWidgets) {
    // Sidebar box
    let graph_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(5)
        .build();
    graph_box.set_margin_all(10);

    // Generate graph button with spinner
    let generate_button_box = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    generate_button_box.set_halign(gtk::Align::Center);
    let generate_button_spinner = gtk::Spinner::builder()
        .spinning(false)
        .visible(false)
        .build();
    generate_button_box.append(&gtk::Label::new(Some("Generate graph")));
    generate_button_box.append(&generate_button_spinner);
    let generate_graph_button = gtk::Button::builder()
        .css_classes(vec!["suggested-action".to_string()])
        .child(&generate_button_box)
        .build();
    generate_graph_button.connect_clicked(clone!(@strong sender => move |_| {
        sender.input(AppMsg::GraphMsg(GraphMsg::Generating(true)))
    }));
    graph_box.append(&generate_graph_button);

    // Graph options
    let graph_options_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    graph_box.append(&graph_options_sep);

    // Output format and DPI
    let output_format_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .hexpand(true)
        .halign(gtk::Align::Center)
        .build();
    let output_format_label = gtk::Label::builder()
        .label("<b>Output format</b>")
        .use_markup(true)
        .build();
    let output_format_dropdown = gtk::DropDown::from_strings(&SUPPORTED_FORMATS);
    output_format_dropdown.connect_selected_item_notify(clone!(@strong sender => move |btn| {
            sender.input(AppMsg::GraphMsg(GraphMsg::SetFileExtension(SUPPORTED_FORMATS[btn.selected() as usize].to_string())));
    }));
    let output_dpi_label = gtk::Label::builder()
        .label("<b>DPI</b>")
        .use_markup(true)
        .build();
    let output_dpi = gtk::Entry::builder()
        .name("DPI")
        .placeholder_text("96.0")
        .tooltip_text("Warning: SVG output might be cropped with incorrect values")
        .build();
    output_dpi.connect_changed(clone!(@strong sender => move |output_dpi| {
        sender.input(AppMsg::GraphMsg(GraphMsg::TrySetOutputDPI(output_dpi.text().to_string())))
    }));
    output_format_box.append(&output_format_label);
    output_format_box.append(&output_format_dropdown);
    output_format_box.append(&output_dpi_label);
    output_format_box.append(&output_dpi);
    graph_box.append(&output_format_box);

    // Layout engine
    let layout_engine_box = gtk::Box::new(gtk::Orientation::Horizontal, 13);
    layout_engine_box.append(
        &gtk::Label::builder()
            .label("<b>Layout engine</b>")
            .use_markup(true)
            .build(),
    );
    let layout_engines: [&str; 4] = [
        (&LayoutEngine::Dot).into(),
        (&LayoutEngine::Neato).into(),
        (&LayoutEngine::Fdp).into(),
        (&LayoutEngine::Circo).into(),
    ];
    let layout_engine_dropbox = gtk::DropDown::from_strings(&layout_engines);
    layout_engine_dropbox.connect_selected_notify(
        clone!(@strong sender, @strong layout_engines => move |dropdown| {
            let index = dropdown.selected();
            if let Some(layout_engine_str) = layout_engines.get(index as usize) {
                if let Ok(layout_engine) = LayoutEngine::from_str(layout_engine_str) {
                    sender.input(AppMsg::GraphMsg(GraphMsg::SetLayoutEngine(layout_engine)))
                }
            }
        }),
    );
    layout_engine_box.append(&layout_engine_dropbox);
    graph_box.append(&layout_engine_box);

    // Checkboxes
    let graph_options = GraphOptions::new();
    let hide_loopback_checkbox = gtk::CheckButton::with_label("Hide loopback connections");
    hide_loopback_checkbox.connect_toggled(
        clone!(@strong sender, @strong graph_options => move |button| {
            sender.input(AppMsg::GraphMsg(GraphMsg::SetHideLoopbackConnections(button.is_active())));
        }),
    );
    graph_box.append(&hide_loopback_checkbox);
    let vertical_graph_checkbox = gtk::CheckButton::with_label("Vertical graph");
    vertical_graph_checkbox.connect_toggled(clone!(@strong sender => move |button| {
        sender.input(AppMsg::GraphMsg(GraphMsg::SetVerticalGraph(button.is_active())));
    }));
    graph_box.append(&vertical_graph_checkbox);
    let transparent_background_checkbox = gtk::CheckButton::with_label("Transparent background");
    transparent_background_checkbox.connect_toggled(clone!(@strong sender => move |button| {
        sender.input(AppMsg::GraphMsg(GraphMsg::SetTransparentBackground(button.is_active())));
    }));
    graph_box.append(&transparent_background_checkbox);
    let hide_legend_checkbox = gtk::CheckButton::with_label("Hide legend");
    hide_legend_checkbox.connect_toggled(clone!(@strong sender => move |button| {
        sender.input(AppMsg::GraphMsg(GraphMsg::SetHideLegend(button.is_active())));
    }));
    graph_box.append(&hide_legend_checkbox);
    let hide_agents_checkbox = gtk::CheckButton::with_label("Hide agents");
    hide_agents_checkbox.set_active(true);
    hide_agents_checkbox.connect_toggled(clone!(@strong sender => move |button| {
        sender.input(AppMsg::GraphMsg(GraphMsg::SetHideAgents(button.is_active())));
    }));
    graph_box.append(&hide_agents_checkbox);

    // Add to the view stack
    sidebar_stack.add(&graph_box);
    sidebar_stack
        .page(&graph_box)
        .set_icon_name(Some("emblem-shared-symbolic"));
    sidebar_stack.page(&graph_box).set_title(Some("Graph"));

    // Leaflet separator
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    flap.set_separator(Some(&separator));

    // Leaflet content
    let leaflet_content = gtk::Box::builder()
        .hexpand(true)
        .vexpand(true)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .build();
    let image_view_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    let image_preview_placeholder = adw::StatusPage::builder()
        .title("Graph preview")
        .description("Once generated, the graph will appear here")
        .width_request(300)
        .height_request(400)
        .icon_name("view-app-grid")
        .build();
    image_view_stack.add_child(&image_preview_placeholder);

    // Image
    let graph_image = gtk::Picture::new();
    graph_image.set_hexpand(true);
    graph_image.set_vexpand(true);
    graph_image.set_can_shrink(true);

    // Add to stack
    image_view_stack.add_child(&graph_image);
    image_view_stack.set_visible_child(&image_preview_placeholder);

    leaflet_content.append(&image_view_stack);
    flap.set_content(Some(&leaflet_content));

    let graph_page_widgets = GraphPageWidgets {
        generate_button_spinner,
        image_view_stack,
        graph_image,
    };
    (graph_options, graph_page_widgets)
}
