//! This module manages the CLI arguments API

use clap::Parser;
use sockets_map::graphviz::LayoutEngine;

#[derive(Parser)]
#[clap(version = clap::crate_version!(), author = "Aurelien Dubois <aurelien.dubois@amossys.fr>", about = "A tool to map the network interactions between processes in a group of machines, from information that can be gathered using native tools on the targets.")]
pub struct Opts {
    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,

    #[clap(subcommand)]
    subcmd: SubCommand,
}

impl Opts {
    /// Get a reference to the opts's verbose.
    pub fn verbose(&self) -> u32 {
        self.verbose
    }

    /// Get a reference to the opts's subcmd.
    pub fn subcmd(&self) -> &SubCommand {
        &self.subcmd
    }
}

#[derive(Parser)]
pub enum SubCommand {
    #[clap(about = "Generate a Graphviz graph of the targets' processes interconnections")]
    Graph(Graph),
    #[clap(about = "Output a CSV with all compiled information about the targets")]
    Csv(Csv),
    #[clap(
        about = "Show cheatsheets to gather information about targets to use with this program"
    )]
    Cheatsheet(Cheatsheet),
}

#[derive(Parser)]
pub struct Graph {
    #[clap(long = "no-loopback", help = "Do not display loopback connections")]
    no_loopback: bool,
    #[clap(
        long = "vertical",
        help = "Arrange tho hosts vertically instead of horizontally"
    )]
    vertical: bool,
    #[clap(
        long = "transparent-bg",
        help = "Use a transparent background instead of plain white"
    )]
    transparent_bg: bool,
    #[clap(long = "hide-legend", help = "Hide the legend")]
    hide_legend: bool,
    #[clap(long = "dump", help = "Dump dot code to file")]
    dump: Option<std::path::PathBuf>,
    #[clap(help = "Graph output file (extension will be passed to Graphviz")]
    output_file: std::path::PathBuf,
    #[clap(help = "Directory containing the files for the hosts to include in the analysis")]
    files_directory: std::path::PathBuf,
    #[clap(
        long = "dpi",
        help = "DPI value for the graph (DPI other than 96 may give strange results for SVG output)"
    )]
    dpi: Option<f64>,
    #[clap(
        help = "Layout engine to use (dot, neato, fdp, sfdp, circo, twopi, osage or patchwork)"
    )]
    layout_engine: Option<LayoutEngine>,
}

impl Graph {
    /// Get a reference to the graph's no loopback.
    pub fn no_loopback(&self) -> bool {
        self.no_loopback
    }

    /// Get a reference to the graph's vertical.
    pub fn vertical(&self) -> bool {
        self.vertical
    }

    /// Get a reference to the graph's dump.
    pub fn dump(&self) -> Option<&std::path::PathBuf> {
        self.dump.as_ref()
    }

    /// Get a reference to the graph's output file.
    pub fn output_file(&self) -> &std::path::PathBuf {
        &self.output_file
    }

    /// Get a reference to the graph's files directory.
    pub fn files_directory(&self) -> &std::path::PathBuf {
        &self.files_directory
    }

    /// Get a reference to the graph's transparent background setting.
    pub fn transparent_bg(&self) -> bool {
        self.transparent_bg
    }

    /// Get a reference to the graph's hide legend settings.
    pub fn hide_legend(&self) -> bool {
        self.hide_legend
    }

    // / Get a reference to the graph's dpi setting.
    pub fn dpi(&self) -> Option<f64> {
        self.dpi
    }

    // / Get a reference to the graph's layout engine setting.
    pub fn layout_engine(&self) -> Option<&LayoutEngine> {
        self.layout_engine.as_ref()
    }
}

#[derive(Parser)]
pub struct Csv {
    #[clap(help = "CSV output file")]
    output_file: std::path::PathBuf,
    #[clap(help = "Directory containing the files for the hosts to include in the analysis")]
    files_directory: std::path::PathBuf,
}

impl Csv {
    /// Get a reference to the csv's files directory.
    pub fn files_directory(&self) -> &std::path::PathBuf {
        &self.files_directory
    }

    /// Get a reference to the csv's output file.
    #[must_use]
    pub fn output_file(&self) -> &std::path::PathBuf {
        &self.output_file
    }
}

#[derive(Parser)]
pub struct Cheatsheet {
    #[clap(subcommand)]
    smbcmd: CheatsheetSubcommand,
}

impl Cheatsheet {
    /// Get a reference to the help's smbcmd.
    pub fn smbcmd(&self) -> &CheatsheetSubcommand {
        &self.smbcmd
    }
}

#[derive(Parser)]
pub enum CheatsheetSubcommand {
    #[clap(about = "Show how to make captures on Linux hosts")]
    Linux,
    #[clap(about = "Show how to make captures on Windows hosts")]
    Windows,
    #[clap(about = "Show how to make captures on remote hosts using nmap")]
    UnknownRemote,
    #[clap(about = "Show how to craft a CSV to simulate a capture")]
    Csv,
    #[clap(about = "Show a summary about how to use the tool")]
    Summary,
    #[clap(about = "Show how to make captures for all types of hosts, in a pager")]
    All,
}
