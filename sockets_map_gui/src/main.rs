// Do not show console on Windows
#![windows_subsystem = "windows"]

use relm4::RelmApp;
use ui::AppModel;

mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logger
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("unable to init termlogger");

    // GUI
    let app = RelmApp::new("fr.amossys.socketsmap");
    app.run::<AppModel>(());

    Ok(())
}
