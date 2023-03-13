use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser)]
#[clap(version = clap::crate_version!(), author = "Aurelien Dubois <aurelien.dubois@amossys.fr>", about = "A tool to connect to a Socket Map server in order to map the network interactions between processes in a group of machines, from information that can be gathered using native tools on the targets.")]
pub struct Args {
    #[clap(help = "address:port of the sockets map server")]
    pub address: SocketAddr,
    #[clap(help = "name to display in the graph for this host")]
    pub pretty_name: Option<String>,
    #[clap(
        help = "run without root privileges (not all processes will be shown !)",
        short = 'n',
        long = "no-root",
        action
    )]
    pub no_root: bool,
}
