# For Linux hosts

To capture information about a **Linux host**, you need to save the output of the `ip` and `ss` or `netstat` commands to files with the following format:
- `<machine 1 name>.linux_ip`
- `<machine 1 name>.linux_ss` or `<machine 1 name>.linux_netstat`
- …

In order to make these files, execute the following commands on the target Linux hosts, **as root**:

> **If running the capture on a non-English system, run `export LC_ALL=C` beforehand.**

- `ss` commands:
    - for an instantaneous capture:
        ```bash
        ss -apn > $(hostname).ss
        ```

    - for an unlimited capture:
        ```bash
        while true; do ss -apn >> /tmp/$(hostname).ss_long; sleep 0.5; done
        cat /tmp/$(hostname).ss_long | sort | uniq > $(hostname).ss
        rm /tmp/$(hostname).ss_long
        ```

- `netstat` commands (if `ss` is not available):
    - for an instantaneous capture:
        ```bash
        (netstat -Wltpn; netstat -Wtpn) > $(hostname).linux_netstat
        ```

    - for an unlimited capture:
        ```bash
        while true; do (netstat -Wltpn; netstat -Wtpn) >> /tmp/$(hostname).netstat_original; sleep 0.5; done
        cat /tmp/$(hostname).netstat_original | sort | uniq > $(hostname).linux_netstat
        rm /tmp/$(hostname).netstat_original
        ```

- `ip` command:
    ```bash
    ip a > $(hostname).linux_ip
    ```
