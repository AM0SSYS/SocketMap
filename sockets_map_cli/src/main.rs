use std::path::Path;

use clap::Parser;

mod cli_args;
mod help;
use sockets_map::{connections_model, csv, graphs, graphviz, parsers};

#[tokio::main]
async fn main() {
    // Help message
    let help = help::HelpMessages::default();

    // Parse arguments
    let args = cli_args::Opts::parse();

    // Initialize logger
    let log_level = match args.verbose() {
        0 => simplelog::LevelFilter::Error,
        1 => simplelog::LevelFilter::Info,
        _ => simplelog::LevelFilter::Debug,
    };
    simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("failed to initialize termlogger");

    // Process subcommand
    match args.subcmd() {
        cli_args::SubCommand::Graph(graph_args) => {
            // Build the Hosts structures
            let scan_dir = graph_args.files_directory();
            let scanned_hosts = parsers::directory_scanner::scan_dir(scan_dir);
            let hosts =
                parsers::directory_scanner::build_hosts(&scanned_hosts).unwrap_or_else(|e| {
                    log::error!("{}", e);
                    std::process::exit(1)
                });

            // Generate connections
            let connections =
                connections_model::build_connections_list(&hosts, graph_args.no_loopback());

            // Parse output file extension
            let output_file_path = graph_args.output_file();
            let extension = output_file_path
                .extension()
                .expect("the output file needs an extension to pass to Graphviz");

            // Generate the Dot graph
            let graph = match graphs::create_graph(
                &connections,
                graph_args.transparent_bg(),
                graph_args.hide_legend(),
                graph_args.dpi().unwrap_or(96.0),
                graph_args.layout_engine(),
            ) {
                Ok(g) => g,
                Err(e) => {
                    log::error!("unable to generate graph: {}", e);
                    std::process::exit(1);
                }
            };

            // Run Graphviz command to generate the graph
            match graphviz::run_graphviz(
                graph.to_string(),
                Path::new(output_file_path),
                extension.to_string_lossy().to_string(),
                graph_args.dump(),
                graph_args.vertical(),
                graph_args.layout_engine(),
            ) {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Error in graph generation: {}", e);
                }
            };
        }
        cli_args::SubCommand::Csv(csv_args) => {
            // Build the Hosts structures
            let scan_dir = csv_args.files_directory();
            let scanned_hosts = parsers::directory_scanner::scan_dir(scan_dir);
            let hosts =
                parsers::directory_scanner::build_hosts(&scanned_hosts).unwrap_or_else(|e| {
                    log::error!("{}", e);
                    std::process::exit(1)
                });

            // Generate connections
            let connections = connections_model::build_connections_list(&hosts, false);

            match csv::write_connections_to_csv(&connections, csv_args.output_file()) {
                Ok(_) => (),
                Err(e) => {
                    log::error!("{}", e);
                }
            };
        }
        cli_args::SubCommand::Cheatsheet(help_args) => {
            match help_args.smbcmd() {
                cli_args::CheatsheetSubcommand::Linux => {
                    help.print_linux();
                }
                cli_args::CheatsheetSubcommand::Windows => {
                    help.print_windows();
                }
                cli_args::CheatsheetSubcommand::UnknownRemote => {
                    help.print_unknown_remote();
                }
                cli_args::CheatsheetSubcommand::Csv => {
                    help.print_csv();
                }
                cli_args::CheatsheetSubcommand::Summary => {
                    help.print_summary();
                }
                cli_args::CheatsheetSubcommand::All => {
                    help.print_all();
                }
            };
        }
    };
}
