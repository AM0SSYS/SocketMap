//! This module models the connections between processesm with listening and connected sockets.

use crate::host;
use log;

#[derive(Debug)]
/// A connection between the connected_host on the connected_connection's local_socket to the
/// listening_host's listening_connection's socket
pub struct Connection<'a> {
    listening_host: &'a host::Host,
    connected_host: &'a host::Host,
    listening_connection: &'a host::ListeningSocket,
    connected_connection: &'a host::Connection,
}

impl<'a> std::fmt::Display for Connection<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} ({} {}:{}) -> {} ({} {}:{})",
            self.connected_host().name(),
            self.connected_connection().process().name(),
            self.connected_connection().local_socket().ip(),
            self.connected_connection().local_socket().port(),
            self.listening_host().name(),
            self.listening_connection().process().name(),
            self.listening_connection().socket().ip(),
            self.listening_connection().socket().port(),
        )
    }
}

impl<'a> Connection<'a> {
    pub fn new(
        listening_host: &'a host::Host,
        connected_host: &'a host::Host,
        listening_connection: &'a host::ListeningSocket,
        connected_connection: &'a host::Connection,
    ) -> Self {
        Self {
            listening_host,
            connected_host,
            listening_connection,
            connected_connection,
        }
    }

    /// Get a reference to the connection's listening host.
    pub fn listening_host(&self) -> &&'a host::Host {
        &self.listening_host
    }

    /// Get a reference to the connection's connected host.
    pub fn connected_host(&self) -> &&'a host::Host {
        &self.connected_host
    }
    /// Get a reference to the connection's connected connection.
    pub fn connected_connection(&self) -> &&'a host::Connection {
        &self.connected_connection
    }

    /// Get a reference to the connection's listening connection.
    pub fn listening_connection(&self) -> &&'a host::ListeningSocket {
        &self.listening_connection
    }
}

/// Build the list of connections between hosts
pub fn build_connections_list(hosts: &[host::Host], no_loopback: bool) -> Vec<Connection<'_>> {
    log::debug!("Building connections list");
    let mut hosts_connections: Vec<Connection> = Vec::new();

    // First, get loopback connection
    if !no_loopback {
        for host in hosts {
            for host_connection in host.connections() {
                for listening_socket in host.listening_sockets() {
                    if host_connection.socket_type() == listening_socket.socket_type()
                        && host_connection.peer_socket().port() == listening_socket.port()
                        && host.ips().contains(&host_connection.peer_socket().ip())
                        && ((host_connection.peer_socket().is_ipv4()
                            && match listening_socket.ipv6_only() {
                                Some(b) => !b,
                                None => false,
                            })
                            || (host_connection.peer_socket().is_ipv6()
                                && listening_socket.ip_addr().is_ipv6())
                            || (host_connection.peer_socket().is_ipv4()
                                && listening_socket.ip_addr().is_ipv4()))
                    {
                        // Here we found a connection between a local process and a local listening
                        // socket
                        let connection =
                            Connection::new(host, host, listening_socket, host_connection);
                        log::debug!("found connection: {}", connection);
                        hosts_connections.push(connection);
                    }
                }
            }
        }
    }

    // Then, get connections between hosts
    for host in hosts {
        for peer in hosts {
            // Skip current host
            if host.name() == peer.name() {
                continue;
            }

            // Loop trough the peer listening sockets
            for peer_listening_socket in peer.listening_sockets() {
                for host_connection in host.connections() {
                    // Check if the connection matches a listening socket
                    if host_connection.socket_type() == peer_listening_socket.socket_type()
                        && peer.ips().contains(&host_connection.peer_socket().ip())
                        && peer_listening_socket.port() == host_connection.peer_socket().port()
                        && ((host_connection.peer_socket().is_ipv4()
                            && match peer_listening_socket.ipv6_only() {
                                Some(b) => !b,
                                None => false,
                            })
                            || (peer_listening_socket.ip_addr().is_ipv4()
                                && host_connection.peer_socket().is_ipv4())
                            || (peer_listening_socket.ip_addr().is_ipv6()
                                && host_connection.peer_socket().is_ipv6()))
                        && !peer_listening_socket.is_loopback()
                    {
                        // Here we found a connection between host and peer, with peer being the
                        // one listening
                        let connection =
                            Connection::new(peer, host, peer_listening_socket, host_connection);
                        log::debug!("found connection: {}", connection);
                        log::debug!(
                            "Peers:\npeer: {:#?}\nhost: {:#?}",
                            peer_listening_socket,
                            host_connection
                        );
                        hosts_connections.push(connection);
                    }
                }
            }

            // Loop through the connected connection to catch sockets that have been handed out to
            // another processes on connection
            for peer_connection in peer.connections() {
                for host_connection in host.connections() {
                    if host_connection.socket_type() == peer_connection.socket_type()
                        && peer.ips().contains(&host_connection.peer_socket().ip())
                        && !host_connection.local_socket().ip().is_loopback()
                        && host_connection.peer_socket().port()
                            == peer_connection.local_socket().port()
                        && host_connection.peer_socket().ip() == peer_connection.local_socket().ip()
                        && ((peer_connection.peer_socket().is_ipv4()
                            && host_connection.peer_socket().is_ipv4())
                            || (peer_connection.local_socket().is_ipv6()
                                && host_connection.peer_socket().is_ipv6()))
                    {
                        // Find the listening socket that peer_connection belongs to
                        let mut connected_peer_listening_socket: Option<&host::ListeningSocket> =
                            None;
                        for peer_listening_socket in peer.listening_sockets() {
                            if peer_connection.local_socket().port() == peer_listening_socket.port()
                            {
                                connected_peer_listening_socket = Some(peer_listening_socket);
                            }
                        }
                        // Here we found a connection between host and peer, with peer being the
                        // one listening
                        if let Some(p) = connected_peer_listening_socket {
                            let connection = Connection::new(peer, host, p, host_connection);
                            log::debug!("found connection: {}", connection);
                            hosts_connections.push(connection);
                        };
                    }
                }
            }
        }
    }

    hosts_connections
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};

    use crate::{
        connections_model::build_connections_list,
        host::{Connection, Host, ListeningSocket, Process, SocketType},
    };

    fn make_fake_connections() -> Vec<Host> {
        // Create machines
        let mut hosts = Vec::<Host>::new();

        // Machine 1
        let mut machine1 = Host::new("machine1");
        // SSHD server
        let sshd_listening_socket = ListeningSocket::new(
            SocketAddr::V6("[::ffff:10.0.0.1]:22".parse().unwrap()),
            SocketType::TCP,
            Process::new("sshd", 101, "machine1".to_string()),
            "machine1".to_string(),
            Some(false),
        );
        // Nginx server
        let nginx_listening_socket = ListeningSocket::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 443)),
            SocketType::TCP,
            Process::new("nginx", 102, "machine1".to_string()),
            "machine1".to_string(),
            None,
        );
        machine1.add_listening_socket(sshd_listening_socket);
        machine1.add_listening_socket(nginx_listening_socket);
        machine1.add_ip(std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));

        let mut machine2 = Host::new("machine2");
        machine2.add_ip(std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)));
        // SSH connection
        machine2.add_established_connection(Connection::new(
            "10.0.0.2:5688".parse().unwrap(),
            "[::ffff:10.0.0.1]:22".parse().unwrap(),
            SocketType::TCP,
            Process::new("ssh", 201, "machine2".to_string()),
        ));
        // HTTPS connection
        machine2.add_established_connection(Connection::new(
            "10.0.0.2:5681".parse().unwrap(),
            "10.0.0.1:443".parse().unwrap(),
            SocketType::TCP,
            Process::new("firefox", 202, "machine2".to_string()),
        ));
        // Some UDP service
        let some_udp_service = ListeningSocket::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 3), 50001)),
            SocketType::UDP,
            Process::new("some_udp_service", 203, "machine2".to_string()),
            "machine2".to_string(),
            None,
        );
        machine2.add_listening_socket(some_udp_service);

        // Machine 3
        let mut machine3 = Host::new("machine3");
        machine3.add_ip(std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)));
        // Some UDP client
        let some_udp_client = Connection::new(
            "10.0.0.3:50002".parse().unwrap(),
            "10.0.0.2:50001".parse().unwrap(),
            SocketType::UDP,
            Process::new("some_udp_client", 301, "machine3".to_string()),
        );
        machine3.add_established_connection(some_udp_client);

        // Machines vector
        hosts.push(machine1);
        hosts.push(machine2);
        hosts.push(machine3);

        hosts
    }

    #[test]
    /// Test TCP connections detection between two IPv4 sockets
    fn test_tcp4_connections() {
        // Make fake connections
        let hosts = make_fake_connections();

        // Build connections list
        let connections = build_connections_list(&hosts, false);

        // Check connection between FireFox and Nginx
        assert!(
            connections.iter().any(|c| {
                // Listening host
                c.listening_host().name() == "machine1"
                    && c.listening_connection().port() == 443
                    && c.listening_connection().process().name() == "nginx"
                    && c.listening_connection().process().pid() == &102
                    && !c.listening_connection().socket().is_ipv6()
                    && c.listening_connection().socket().ip()
                        == "0.0.0.0".parse::<Ipv4Addr>().unwrap()
                // Connected host
                    && c.connected_host().name() == "machine2"
                    && c.connected_connection().process().name() == "firefox"
                    && c.connected_connection().process().pid() == &202
            }),
            "missing TCP connection from machine2 firefox process on machine1 nginx server:\n{connections:#?}"
        );
    }

    #[test]
    /// Test UDP connections detection between two IPv4 sockets
    fn test_udp4_connections() {
        // Make fake connections
        let hosts = make_fake_connections();

        // Build connections list
        let connections = build_connections_list(&hosts, false);

        // Check connection between the UDP client and server
        assert!(
            connections.iter().any(|c| {
                // Listening host
                c.listening_host().name() == "machine2"
                    && c.listening_connection().port() == 50001
                    && c.listening_connection().process().name() == "some_udp_service"
                    && c.listening_connection().process().pid() == &203
                    && !c.listening_connection().socket().is_ipv6()
                    && c.listening_connection().socket().ip()
                        == "10.0.0.3".parse::<Ipv4Addr>().unwrap()
                // Connected host
                    && c.connected_host().name() == "machine3"
                    && c.connected_connection().process().name() == "some_udp_client"
                    && c.connected_connection().process().pid() == &301
            }),
            "missing UDP connection from machine2 to machine3:\n{connections:#?}"
        );
    }

    #[test]
    /// Test TCP connections detection between an IPv6 server with IP6ONLY flag set to false, and
    /// an IPv4 client
    fn test_tcp6_to_4_connections() {
        // Make fake connections
        let hosts = make_fake_connections();

        // Build connections list
        let connections = build_connections_list(&hosts, false);

        // Check connection between SSH and SSHD
        assert!(
            connections.iter().any(|c| {
                // Listening host
                c.listening_host().name() == "machine1"
                    && !c.listening_connection().is_loopback()
                    && c.listening_connection().port() == 22
                    && c.listening_connection().process().name() == "sshd"
                    && c.listening_connection().process().pid() == &101
                    && c.listening_connection().ipv6_only() == Some(&false)
                    && c.listening_connection().socket().is_ipv6()
                    && c.listening_connection().socket().ip()
                        == "::ffff:10.0.0.1".parse::<Ipv6Addr>().unwrap()
                // Connected host
                    && c.connected_host().name() == "machine2"
                    && c.connected_connection().process().name() == "ssh"
                    && c.connected_connection().process().pid() == &201
            }),
            "missing TCP connection from machine2 ssh client on machine1 sshd server:\n{connections:#?}"
        );
    }
}
