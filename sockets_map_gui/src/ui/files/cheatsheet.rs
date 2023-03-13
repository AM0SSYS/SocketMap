//! Cheatsheet help window for files input

use gtk::{
    glib::clone,
    traits::{BoxExt, GtkWindowExt, WidgetExt},
};
use gtk4_commonmark::{self, RenderConfig};
use relm4::{adw, ComponentParts, RelmContainerExt, RelmWidgetExt, SimpleComponent};

#[tracker::track]
pub struct CheatsheetWindow {
    visible: bool,
}

#[derive(Debug)]
pub enum CheatsheetWindowMsg {
    Show,
    Hide,
}

pub struct CheatsheetWindowWidgets {
    root: adw::Window,
}

impl SimpleComponent for CheatsheetWindow {
    type Input = CheatsheetWindowMsg;
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = CheatsheetWindowWidgets;

    fn init_root() -> Self::Root {
        adw::Window::builder()
            .default_height(800)
            .default_width(1200)
            .title("Sockets map cheatsheet")
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
            sender.input(CheatsheetWindowMsg::Hide);
            gtk::Inhibit(true)
        }));

        // Outer box
        let outer_box = gtk::Box::new(gtk::Orientation::Vertical, 5);

        // Stack
        let stack = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::SlideUpDown)
            .build();
        let stack_sidebar = gtk::StackSidebar::builder()
            .stack(&stack)
            .vexpand(true)
            .build();
        let sidebar_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .vexpand(true)
            .build();
        sidebar_box.append(
            &adw::HeaderBar::builder()
                .title_widget(&adw::WindowTitle::new("Pages", ""))
                .show_end_title_buttons(false)
                .build(),
        );
        sidebar_box.append(&stack_sidebar);

        // Header
        let header = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("Usage cheatsheets", ""))
            .show_end_title_buttons(true)
            .build();
        outer_box.append(&header);

        // Flap
        let flap = adw::Flap::new();
        flap.set_content(Some(&outer_box));
        flap.set_flap(Some(&sidebar_box));

        // Markdown pages
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 5);
        content_box.append(&stack);

        add_help_pages(stack);

        outer_box.append(&content_box);
        root.container_add(&flap);

        ComponentParts {
            model: CheatsheetWindow {
                visible: false,
                tracker: 0,
            },
            widgets: CheatsheetWindowWidgets { root: root.clone() },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: relm4::ComponentSender<Self>) {
        self.reset();
        match message {
            CheatsheetWindowMsg::Show => self.set_visible(true),
            CheatsheetWindowMsg::Hide => self.set_visible(false),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: relm4::ComponentSender<Self>) {
        if self.changed(Self::visible()) {
            widgets.root.set_visible(*self.get_visible())
        }
    }
}

fn add_help_pages(stack: gtk::Stack) {
    let help_pages = [
        ("Summary", sockets_map::help::SUMMARY_HELP),
        ("Linux", sockets_map::help::LINUX_HELP),
        ("Windows", sockets_map::help::WINDOWS_HELP),
        ("Unknown remote", sockets_map::help::UNKNOWN_REMOTE_HELP),
        ("CSV", sockets_map::help::CSV_HELP),
    ];

    for (page_name, content) in help_pages {
        let scrollable = gtk::ScrolledWindow::new();
        scrollable.set_hscrollbar_policy(gtk::PolicyType::Automatic);
        scrollable.set_margin_all(10);
        let viewport = gtk4_commonmark::render_input(content, RenderConfig::default())
            .unwrap_or_else(|_| panic!("issue while trying to render {page_name} help page"));
        scrollable.container_add(&viewport);
        stack.add_titled(&scrollable, Some(page_name), page_name);
    }
}
