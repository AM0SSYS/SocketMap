# Custom CSV

 If you want to simulate a machine, you can provide two CSV files describing its IP configuration and its sockets:

- `<machine name>_ip.csv` with the IP addresses of the host (only one column named \"IP\")
- `<machine name>_network.csv` with the socket records of the host. The columns are :
    - `protocol`: values can be TCP, UDP
    - `local_socket`: value is an IP:port association such as `10.0.0.1:22`
    - `foreign_socket`: same
    - `state`: ESTABLISHED or LISTENING
    - `pid`: (not really important, only used when making a summary of the connections)
    - `process_name`

## Example

```csv
protocol, local_socket,   foreign_socket,  state,       pid, process_name
-------------------------------------------------------------------------
tcp,      10.0.0.13:22,   10.0.0.11:53293, ESTABLISHED, 0,   sshd
tcp,      0.0.0.0:22,     ,                LISTENING,   0,   sshd
tcp,      10.0.0.13:5569, 10.0.0.10:3389,  ESTABLISHED, 0,   Remmina-rdp
tcp,      0.0.0.0:53,     ,                LISTENING,   0,   dnsmasq
udp,      10.0.0.13:5353, 10.0.0.1:53,     ESTABLISHED, 0,   dnsmasq_udp
udp,      0.0.0.0:5353,   ,                LISTENING,   0,   dnsmasq_udp
```

> IPv6 sockets format must match what has been defined in [RFC2732](https://www.ietf.org/rfc/rfc2732.txt)
