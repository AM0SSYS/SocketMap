# Summary

This tool's preferred method of data collection is to use the **agents** on the target machines.

However, for hosts that do not allow running these agents, or for manually crafted data, the collection can be exported to files that can be used here later on.

> Agents can be used alongside statically collected data.

## Data collection method

To include static hosts data into the graph, you first need to gather data from the target machines.

For each target machine onto which you can run basic utilities:

- The first file contains network interfaces information. It is the **output of the `ip`** for **Linux** Hosts and **`Get-NetIpaddress`** for **Windows** hosts.
- The second file contains open sockets information. It is the **output of the `ss` or `netstat` command** for **Linux** hosts, and **`netstat` command** for **Windows** hosts.

For machines onto which you cannot run basic utilities:

- You can **scan a remote host using `nmap`** and only provide the command output in a file. It is only recommended when no local access is
available on the remote host, as it comes with less information. Estimated service names will have a `?` at their end in the graph. See the "Unknown remote" cheatsheet for more information.
- You can **manually craft two CSV** files per host. See the "CSV" cheatsheet to know more about this feature.

Once you have all this files, put them inside a separate folder and use this tool to analyze it all.

## Architecture of the collected data folder

A valid folder could be made of the following files, by example:

- `centos.linux_ip`
- `centos.ss`
- `debian_ip.csv`
- `debian_network.csv`
- `linux_server.nmap_10.0.0.254`
- `Windows_Server.windows_ip`
- `Windows_Server.windows_netstat`
- `Windows_Server.windows_tasklist`
- `Windows.windows_ip`
- `Windows.windows_netstat`
- `Windows.windows_tasklist`
