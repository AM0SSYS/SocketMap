//! This module models the DOT objects in order to draw the graph using Graphviz.

use crate::graphviz::LayoutEngine;
use crate::{connections_model, host};
use anyhow::anyhow;
use rand::prelude::ThreadRng;
use rand::Rng;
use tabbycat::attributes::*;
use tabbycat::{AttrList, Edge, GraphType, Identity, StmtList};

const DEFAULT_FONTNAME: &str = "Verdata";

/// The structure to graphically represent a host
pub struct GraphHost<'a> {
    /// The host name
    name: &'a str,
    /// A unique node id
    node_id: &'a str,
    listening_processes_nodes_ids: Vec<&'a str>,
    /// The listening processes nodes associated with this host node
    listening_processes_nodes_stmts: Vec<tabbycat::Stmt<'a>>,
    /// The listening edges nodes associated with this host node
    listening_processes_edges_stmts: Vec<tabbycat::Stmt<'a>>,
    connected_and_listening_processes_nodes_ids: Vec<(&'a str, &'a str)>,
    connected_processes_nodes_ids: Vec<&'a str>,
    connected_processes_edges_stmts: Vec<tabbycat::Stmt<'a>>,
    connected_processes_nodes_stmts: Vec<tabbycat::Stmt<'a>>,
}

impl<'a> GraphHost<'a> {
    pub fn new(name: &'a str, node_id: &'a str) -> Self {
        Self {
            name,
            node_id,
            listening_processes_nodes_ids: Vec::new(),
            listening_processes_nodes_stmts: Vec::new(),
            listening_processes_edges_stmts: Vec::new(),
            connected_and_listening_processes_nodes_ids: Vec::new(),
            connected_processes_nodes_stmts: Vec::new(),
            connected_processes_edges_stmts: Vec::new(),
            connected_processes_nodes_ids: Vec::new(),
        }
    }

    /// Get a reference to the graph host's name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    // Add a listening process and build its statement list
    pub fn add_listening_process(
        &mut self,
        listening_connection: &'a host::ListeningSocket,
        host: &'a host::Host,
    ) {
        let node_id = listening_connection.node_id();

        // See if we already have seen that listening process
        // Only add it to the statements list we we haven't already
        if !self.listening_processes_nodes_ids.contains(&node_id) {
            self.listening_processes_nodes_ids.push(node_id);

            // Build the listening process statements list
            let id = Identity::String(node_id);
            let listening_process_node = tabbycat::Stmt::Node {
                id,
                port: None,
                attr: Some(listening_process_node_attrs(
                    listening_connection.node_name(),
                )),
            };
            let listening_process_edge = tabbycat::Stmt::Edge(
                Edge::head_node(Identity::String(host.cluster_id()), None)
                    .arrow_to_node(Identity::String(listening_connection.node_id()), None)
                    .add_attrpair(color(Color::Black))
                    .add_attrpair(style(Style::Dashed)),
            );
            self.listening_processes_nodes_stmts
                .push(listening_process_node);
            self.listening_processes_edges_stmts
                .push(listening_process_edge);
        }
    }

    // Add a connected process
    pub fn add_connected_process(
        &mut self,
        connected_connection: &'a host::Connection,
        host: &'a host::Host,
        listening_connection: &'a host::ListeningSocket,
        rng: &mut ThreadRng,
    ) {
        let connected_node_id = connected_connection.process().node_id();
        let listening_node_id = listening_connection.node_id();

        // See if we already have seen that connected process
        // Only add it to the statements list we we haven't already
        if !self
            .connected_and_listening_processes_nodes_ids
            .contains(&(connected_node_id, listening_node_id))
        {
            self.connected_and_listening_processes_nodes_ids
                .push((connected_node_id, listening_node_id));

            // Build the connected process statements list
            let connected_process_node = tabbycat::Stmt::Node {
                id: Identity::String(connected_node_id),
                port: None,
                attr: Some(connected_process_node_attrs(
                    connected_connection.process().name(),
                )),
            };

            // Each edge will have a random dark color
            let hue: f32 = rng.gen_range(0.0..1.0);
            let saturation: f32 = rng.gen_range(0.7..0.99);
            let value: f32 = 0.65;

            let interprocess_edge = tabbycat::Stmt::Edge(
                Edge::head_node(
                    Identity::String(connected_connection.process().node_id()),
                    None,
                )
                .arrow_to_node(Identity::String(listening_connection.node_id()), None)
                .add_attrpair(color(Color::HSV(hue, saturation, value))),
            );

            // Check if we already have a link between this host and this connected process
            if !self
                .connected_processes_nodes_ids
                .contains(&connected_connection.process().node_id())
            {
                let connected_process_edge = tabbycat::Stmt::Edge(
                    Edge::head_node(Identity::String(host.cluster_id()), None)
                        .arrow_to_node(
                            Identity::String(connected_connection.process().node_id()),
                            None,
                        )
                        .add_attrpair(color(Color::Black))
                        .add_attrpair(style(Style::Dashed)),
                );
                self.connected_processes_nodes_ids
                    .push(connected_connection.process().node_id());
                self.connected_processes_edges_stmts
                    .push(connected_process_edge);
            }

            self.connected_processes_nodes_stmts
                .push(connected_process_node);
            self.connected_processes_edges_stmts.push(interprocess_edge);
        }
    }

    /// Get a reference to the graph host's listening processes stmts.
    pub fn listening_processes_nodes_stmts(&self) -> Vec<tabbycat::Stmt<'a>> {
        self.listening_processes_nodes_stmts.clone()
    }

    /// Get a reference to the graph host's connected processes stmts.
    pub fn connected_processes_nodes_stmts(&self) -> Vec<tabbycat::Stmt<'a>> {
        self.connected_processes_nodes_stmts.clone()
    }

    /// Get a reference to the graph host's node id.
    pub fn node_id(&self) -> &'a str {
        self.node_id
    }

    /// Get a reference to the graph host's connected processes edges stmts.
    pub fn connected_processes_edges_stmts(&self) -> Vec<tabbycat::Stmt<'a>> {
        self.connected_processes_edges_stmts.clone()
    }

    /// Get a reference to the graph host's listening processes edges stmts.
    pub fn listening_processes_edges_stmts(&self) -> Vec<tabbycat::Stmt<'a>> {
        self.listening_processes_edges_stmts.clone()
    }
}

fn graph_host_node_attrs(name: &str) -> AttrList {
    AttrList::new()
        .add_pair(fontname(DEFAULT_FONTNAME))
        .add_pair(label(name))
        .add_pair(shape(Shape::Egg))
        .add(
            Identity::String("style"),
            Identity::String("\"filled,bold\""),
        )
        .add_pair(fillcolor(Color::White))
}

fn connected_process_node_attrs(name: &str) -> AttrList {
    AttrList::new()
        .add_pair(fontname(DEFAULT_FONTNAME))
        .add_pair(shape(Shape::Box))
        .add(
            Identity::String("style"),
            Identity::String("\"rounded,filled\""),
        )
        .add_pair(fillcolor(Color::White))
        .add_pair(label(name))
}

fn listening_process_node_attrs(name: &str) -> AttrList {
    AttrList::new()
        .add_pair(fontname(DEFAULT_FONTNAME))
        .add_pair(shape(Shape::Box))
        .add_pair((
            Identity::String("style"),
            Identity::String("\"rounded,filled\""),
        ))
        .add_pair(fillcolor(Color::Black))
        .add_pair((Identity::String("fontcolor"), Identity::from(Color::White)))
        .add_pair(label(name))
}

/// Create hosts subgraphs with their connected listening and connected processes around it
fn create_hosts_subgraph<'a>(
    connections: &Vec<connections_model::Connection<'a>>,
) -> (Vec<tabbycat::SubGraph<'a>>, StmtList<'a>) {
    let mut subgraphs: Vec<tabbycat::SubGraph> = Vec::new();
    let mut edges_stmts = tabbycat::StmtList::new();

    // Styling parameters
    let host_subgraph_attrs = tabbycat::StmtList::new().add_attr(
        tabbycat::AttrType::Graph,
        AttrList::new()
            .add_pair(fontname(DEFAULT_FONTNAME))
            .add(
                Identity::String("style"),
                Identity::String("\"rounded,filled\""),
            )
            .add_pair(color(Color::Lightgrey)),
    );

    // Keep track of the hosts we saw during the loop
    let mut graph_hosts: Vec<GraphHost> = Vec::new();

    // Initialize the rng for random edge color generation
    let mut rng = rand::thread_rng();

    for connection in connections {
        let listening_host = connection.listening_host();
        let connected_host = connection.connected_host();
        let listening_connection = connection.listening_connection();
        let connected_connection = connection.connected_connection();

        // See if we already have seen this listening host before
        if !&graph_hosts
            .iter()
            .map(|h| h.name())
            .any(|graph_host_name| graph_host_name == listening_host.name())
        {
            // First time seeing that host, create the GraphHost object
            let graph_host: GraphHost<'a> =
                GraphHost::new(listening_host.name(), listening_host.cluster_id());
            graph_hosts.push(graph_host);
        }

        // Same for the connected host
        if !graph_hosts
            .iter()
            .map(|h| h.name())
            .any(|graph_host_name| graph_host_name == connected_host.name())
        {
            // First time seeing that host, create the GraphHost object
            let graph_host = GraphHost::new(connected_host.name(), connected_host.cluster_id());
            graph_hosts.push(graph_host);
        }

        // Add the listening process to the listening host
        for graph_host in &mut graph_hosts {
            if graph_host.name() == listening_host.name() {
                graph_host.add_listening_process(listening_connection, listening_host);
                break;
            }
        }

        // Add the connected process to the connected host
        for graph_host in &mut graph_hosts {
            if graph_host.name() == connected_host.name() {
                graph_host.add_connected_process(
                    connected_connection,
                    connected_host,
                    listening_connection,
                    &mut rng,
                );
                break;
            }
        }
    }

    // Create the subgraphs from the GraphHost structures
    for graph_host in graph_hosts {
        // Create the StmtList, starting with the host node
        let layout = AttrList::new().add_pair(layout("dot"));
        let mut stmts = tabbycat::StmtList::new()
            .add_node(
                Identity::String(graph_host.node_id()),
                None,
                Some(graph_host_node_attrs(graph_host.name())),
            )
            .extend(host_subgraph_attrs.clone())
            .add_attr(tabbycat::AttrType::Graph, layout.clone());
        for stmt in graph_host.listening_processes_nodes_stmts() {
            stmts = stmts.add(stmt);
        }
        for stmt in graph_host.connected_processes_nodes_stmts() {
            stmts = stmts.add(stmt);
        }
        for stmt in graph_host.listening_processes_edges_stmts() {
            edges_stmts = edges_stmts.add(stmt)
        }
        for stmt in graph_host.connected_processes_edges_stmts() {
            edges_stmts = edges_stmts.add(stmt);
        }
        let subgraph =
            tabbycat::SubGraph::subgraph(Some(Identity::String(graph_host.node_id())), stmts);
        subgraphs.push(subgraph);
    }

    (subgraphs, edges_stmts)
}

// Create the graph
pub fn create_graph<'a>(
    connections: &Vec<connections_model::Connection<'a>>,
    transparent_background: bool,
    hide_legend: bool,
    dpi_value: f64,
    layout_engine: Option<&LayoutEngine>,
) -> anyhow::Result<tabbycat::Graph<'a>> {
    let graph_builder = tabbycat::GraphBuilder::default()
        .graph_type(GraphType::DiGraph)
        .strict(false)
        .id(Identity::String("G"));
    let mut layout = AttrList::new()
        .add_pair(layout(layout_engine.map(|l| l.into()).unwrap_or("dot")))
        .add_pair(fontname(DEFAULT_FONTNAME))
        .add_pair(match layout_engine {
            None => scale(1.0),
            Some(l) => match l {
                LayoutEngine::Neato => scale(2.0),
                LayoutEngine::Fdp => K(1.5),
                LayoutEngine::Circo => scale(1.0),
                LayoutEngine::Dot => scale(1.0),
            },
        });

    // Background
    if transparent_background {
        layout = layout.add_pair(bgcolor(Color::Transparent));
    } else {
        layout = layout.add_pair(bgcolor(Color::White));
    }

    // Hosts subgraphs
    let hosts_subgraphs = create_hosts_subgraph(connections);
    let mut graph_stmts = tabbycat::StmtList::new()
        .add_attr(tabbycat::AttrType::Graph, layout.clone())
        .add_attr(
            tabbycat::AttrType::Graph,
            AttrList::new().add_pair(dpi(dpi_value)),
        );
    for host_subgraph in hosts_subgraphs.0 {
        graph_stmts = graph_stmts.add_subgraph(host_subgraph);
    }
    graph_stmts = graph_stmts.extend(hosts_subgraphs.1);

    // Legend
    if !hide_legend {
        let legend_subgraph = generate_legend();
        graph_stmts = graph_stmts.add_subgraph(legend_subgraph);
    }

    graph_builder
        .stmts(graph_stmts)
        .build()
        .map_err(|e| anyhow!(e))
}

fn generate_legend<'a>() -> tabbycat::SubGraph<'a> {
    // Styling parameters
    let legend_cluster_attrs = tabbycat::StmtList::new()
        .add_attr(
            tabbycat::AttrType::Graph,
            AttrList::new()
                .add_pair(fontname(DEFAULT_FONTNAME))
                .add_pair(label("Legend"))
                .add_pair(fontsize(9.0))
                .add_pair(labeljust("left"))
                .add(
                    Identity::String("style"),
                    Identity::String("\"rounded,filled\""),
                )
                .add_pair(color(Color::Black))
                .add_pair(fillcolor(Color::White))
                .add_pair(rankdir(RankDir::LR))
                .add_pair(ranksep(0.3))
                .add_pair(nodesep(0.3)),
        )
        .add_attr(
            tabbycat::AttrType::Node,
            AttrList::new()
                .add_pair(fontname(DEFAULT_FONTNAME))
                .add_pair(shape(Shape::Box))
                .add_pair(margin(0.01))
                .add_pair(height(0.01))
                .add(
                    Identity::String("style"),
                    Identity::String("\"rounded,filled\""),
                )
                .add_pair(fillcolor(Color::White))
                .add_pair(fontsize(8.0)),
        )
        .add_attr(
            tabbycat::AttrType::Edge,
            AttrList::new().add_pair(arrowsize(0.4)),
        );

    let legend_stmts = tabbycat::StmtList::new()
        .extend(legend_cluster_attrs)
        .add_node(
            Identity::String("host1"),
            None,
            Some(graph_host_node_attrs("Host")),
        )
        .add_node(
            Identity::String("listening_process"),
            None,
            Some(listening_process_node_attrs(
                "Listening process\nprotocol:port",
            )),
        )
        .add_node(
            Identity::String("connected_process"),
            None,
            Some(connected_process_node_attrs("Connected process")),
        )
        .add_edge(
            Edge::head_node(Identity::String("host1"), None)
                .arrow_to_node(Identity::String("listening_process"), None)
                .add_attrpair(style(Style::Dashed)),
        )
        .add_edge(
            Edge::head_node(Identity::String("host1"), None)
                .arrow_to_node(Identity::String("connected_process"), None)
                .add_attrpair(style(Style::Dashed)),
        )
        .add_edge(
            Edge::head_node(Identity::String("connected_process"), None)
                .arrow_to_node(Identity::String("listening_process"), None)
                .add_attrpair(color(Color::Darkblue))
                .add_attrpair(constraint(false)),
        )
        .add_edge(
            Edge::head_node(Identity::String("connected_process"), None)
                .arrow_to_node(Identity::String("listening_process"), None)
                .add_attrpair(color(Color::Darkred))
                .add_attrpair(constraint(false)),
        )
        .add_edge(
            Edge::head_node(Identity::String("connected_process"), None)
                .arrow_to_node(Identity::String("listening_process"), None)
                .add_attrpair(color(Color::Darkgreen))
                .add_attrpair(constraint(false)),
        );
    let legend_subgraph = tabbycat::SubGraph::subgraph(
        Some(tabbycat::Identity::String("cluster_legend")),
        legend_stmts,
    );
    legend_subgraph
}
