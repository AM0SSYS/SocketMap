//! Help window

use gtk::glib::clone;
use gtk::traits::BoxExt;
use gtk::traits::{GtkWindowExt, WidgetExt};
use gtk4_commonmark::{self, RenderConfig};
use relm4::{adw, ComponentParts, RelmContainerExt, RelmWidgetExt, SimpleComponent};

#[tracker::track]
pub struct HelpWindow {
    visible: bool,
}

#[derive(Debug)]
pub enum HelpWindowMsg {
    Show,
    Hide,
}

pub struct HelpWindowWidgets {
    root: adw::Window,
}

impl SimpleComponent for HelpWindow {
    type Input = HelpWindowMsg;
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = HelpWindowWidgets;

    fn init_root() -> Self::Root {
        adw::Window::builder()
            .default_height(800)
            .default_width(1200)
            .title("Sockets map help")
            .decorated(true)
            .visible(false)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.connect_close_request(clone!(@strong sender => move |_| {
            sender.input(HelpWindowMsg::Hide);
            gtk::Inhibit(true)
        }));

        let outer_box = gtk::Box::new(gtk::Orientation::Vertical, 5);

        // Header
        let header = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("Help", ""))
            .show_end_title_buttons(true)
            .build();
        outer_box.append(&header);

        // Help page
        let help_input = include_str!("../../res/help.md");
        let scrollable = gtk::ScrolledWindow::new();
        scrollable.set_margin_all(10);
        let viewport = gtk4_commonmark::render_input(help_input, RenderConfig::default())
            .expect("unable to render README.md as GTK4 widgets");
        scrollable.container_add(&viewport);

        outer_box.append(&scrollable);
        root.container_add(&outer_box);

        ComponentParts {
            model: HelpWindow {
                visible: false,
                tracker: 0,
            },
            widgets: HelpWindowWidgets { root: root.clone() },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: relm4::ComponentSender<Self>) {
        self.reset();
        match message {
            HelpWindowMsg::Show => self.set_visible(true),
            HelpWindowMsg::Hide => self.set_visible(false),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: relm4::ComponentSender<Self>) {
        if self.changed(Self::visible()) {
            widgets.root.set_visible(*self.get_visible())
        }
    }
}
