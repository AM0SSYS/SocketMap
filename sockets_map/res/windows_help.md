# For Windows hosts

To capture information about a **Windows host**, you need to save the output of the `ip` and `netstat` commands to files with the following format:
- `<machine 1 name>.windows_ip`
- `<machine 1 name>.windows_netstat`
- `<machine 1 name>.windows_tasklist`
- …

In order to make these files, execute the following commands on the target Windows hosts, **as Administrator**:
- `netstat` commands:
    - for an instantaneous capture:
        ```bash
        netstat -ano > \"$env:COMPUTERNAME.windows_netstat\"
        ```

    - for an unlimited capture:
        ```bash
        while($true) { netstat -ano >> \"$env:COMPUTERNAME.windows_netstat_long\"; sleep 0.5 }
        cat \"$env:COMPUTERNAME.windows_netstat_long\" | sort | unique > \"$env:COMPUTERNAME.windows_netstat\"
        ```

- `get-netipaddress` command:
    ```bash
    Get-NetIpAddress > \"$env:COMPUTERNAME.windows_ip\"
    ```

- `tasklist` command:
    ```bash
    tasklist /FO CSV > \"$env:COMPUTERNAME.windows_tasklist\"
    ```
