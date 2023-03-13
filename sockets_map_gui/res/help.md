# Sockets Map

A tool to represent, graphically, the network interactions between processes across different computers within the same LAN.

It is useful to quickly analyze software network architectures for security research.

The main difference between this tool and other network mapping tools is the focus on interactions between processes, instead of interactions between sockets only.

# Usage

This tool supports two different methods to gather the data that is necessary to build the graph:

1. Agents can be deployed onto the machines to be included in the graph. They can connect to the server available in the GUI version of the app in order to transmit instant captures of their state, or captures at regular intervals which will be aggregated (recorder mode).
2. For hosts on which you cannot run the agent, a simple collection method based on standard commands output is available. Or, you can simulate machines. This is further described in the GUI "cheatsheet" window, as well as in the CLI "cheatsheet" subcommand.

This tool comes with four components:

- the GUI binary (`sockets_map_gui`) built with GTK4 and libadwaita
- the CLI binary (`sockets_mal_cli`)
- the agent binary which sends collected data to the `sockets_map` server (`sockets_map_agent`)
- the `sockets_map` library shared between the other components (`sockets_map`)

> Warning: this tool was made to run in a lab environment. There is no encryption between the agents and the server. If you want to use this tool in a production environment, be sure to use it over a VPN connection not to expose processes information on your network.

# Usage example with agents from the GUI

To create a graph from agents with the GUI, follow these steps:

1. Navigate to the *Server* tab.
2. Click on the *Start server* button, after setting the server address and port (make sure your firewall allows that TCP traffic!)
3. Start the agents (with root/admin privileges). You'll see them in the *Active clients* section when they connect.
4. Press the *Update* button to make a one-time collect. You can then go back to the *Graph* tab and press *Generate graph*.
5. If you want to make a graph from aggregated collects over a certain period of time, press the *Record* button in the *Server* tab. Press once again to stop the recording and receive the collected data.

# Capabilities

This tool cross-references the collected data to build a connection model. The supported connections are:

- **TCP4** sockets connected to **TCP4** listening sockets
- **TCP6** sockets connected to **TCP6** listening sockets
- **TCP4** sockets connected to **TCP6** listening sockets (with `IPV6_V6ONLY == 0`)
- **UDP4** sockets exclusively bound to **UDP4** listening sockets (Linux only)
- **UDP6** sockets exclusively bound to **UDP6** listening sockets (Linux only)
- **UDP4** sockets exclusively bound to **UDP6** listening sockets (with `IPV6_V6ONLY == 0`) (Linux only)

Connections to sockets that were handed over to other processes on connection are also supported.

> Exclusively bound (aka `ESTABLISHED`) UDP sockets are only available on Linux hosts.
> 
> On Linux, using `ss` is preferred to using `netstat`. Agents will automatically use `ss` if available.
