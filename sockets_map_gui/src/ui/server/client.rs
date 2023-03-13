//! Factory component to display active server clients
use gtk::traits::BoxExt;
use humantime;
use relm4::{
    self,
    prelude::{DynamicIndex, FactoryComponent},
};
use std::{net::IpAddr, time::Duration};

use crate::ui::AppMsg;

#[derive(Debug, PartialEq, Eq)]
pub struct ClientInfo {
    pub hostname: String,
    pub pretty_name: Option<String>,
    pub ips: Vec<IpAddr>,
}

#[derive(Debug)]
#[tracker::track]
pub struct ClientLabel {
    pub info: ClientInfo,
    pub index: DynamicIndex,
    last_update: Option<std::time::Instant>,
    recording: bool,
}

pub struct ClientLabelWidgets {
    last_update_label: gtk::Label,
}

#[derive(Debug, Clone)]
pub enum ClientLabelMsg {
    GotUpdate,
    Recording(bool),
}

#[derive(Debug)]
pub enum ClientLabelCmdOutput {
    LastUpdateTimerTick,
}

impl FactoryComponent for ClientLabel {
    type ParentWidget = gtk::ListBox;
    type ParentInput = AppMsg;
    type CommandOutput = ClientLabelCmdOutput;
    type Input = ClientLabelMsg;
    type Output = ();
    type Init = ClientInfo;
    type Root = gtk::Box;
    type Widgets = ClientLabelWidgets;

    fn init_model(
        init: Self::Init,
        index: &relm4::prelude::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        Self {
            index: index.clone(),
            info: init,
            last_update: None,
            tracker: 0,
            recording: false,
        }
    }

    fn init_root(&self) -> Self::Root {
        gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(20)
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build()
    }

    fn init_widgets(
        &mut self,
        _index: &relm4::prelude::DynamicIndex,
        root: &Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: relm4::FactorySender<Self>,
    ) -> Self::Widgets {
        // Client label
        let text = if let Some(pretty_name) = &self.info.pretty_name {
            format!("{pretty_name} ({})", &self.info.hostname)
        } else {
            self.info.hostname.clone()
        };
        let host_label = gtk::Label::new(Some(&text));

        // Last update label
        let last_update_label = gtk::Label::builder()
            .hexpand(true)
            .halign(gtk::Align::End)
            .use_markup(true)
            .build();

        root.append(&host_label);
        root.append(&last_update_label);

        ClientLabelWidgets { last_update_label }
    }

    fn update(&mut self, message: Self::Input, sender: relm4::FactorySender<Self>) {
        self.reset();
        match message {
            ClientLabelMsg::GotUpdate => {
                self.set_last_update(Some(std::time::Instant::now()));
                sender.oneshot_command(async move { ClientLabelCmdOutput::LastUpdateTimerTick });
            }
            ClientLabelMsg::Recording(recording) => {
                self.set_last_update(None);
                self.set_recording(recording);
                if !recording {}
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: relm4::FactorySender<Self>) {
        if self.changed(Self::last_update()) {
            widgets.last_update_label.set_label(
                "<span size=\"small\" foreground=\"grey\"><i>updated just now ago</i></span>",
            );
        }
        if self.changed(Self::recording()) {
            if *self.get_recording() {
                widgets
                    .last_update_label
                    .set_label("<span size=\"small\" foreground=\"grey\"><i>recording…</i></span>");
            } else {
                widgets.last_update_label.set_label(
                    "<span size=\"small\" foreground=\"grey\"><i>collecting…</i></span>",
                );
            }
        }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: relm4::FactorySender<Self>,
    ) {
        self.reset();
        match message {
            ClientLabelCmdOutput::LastUpdateTimerTick => {
                if let Some(last_update) = self.last_update {
                    let now = std::time::Instant::now();
                    let interval = now - last_update;
                    let interval = Duration::new(interval.as_secs(), 0);
                    widgets.last_update_label.set_label(&format!(
                        "<span size=\"small\" foreground=\"grey\"><i>updated {} ago</i></span>",
                        humantime::format_duration(interval)
                    ));
                    sender.oneshot_command(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        ClientLabelCmdOutput::LastUpdateTimerTick
                    })
                }
            }
        }
    }
}
