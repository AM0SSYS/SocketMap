# Unknown or inaccessible remote host

If you do not have a local shell on the remote host, you can scan it with `nmap` and pipe the output to a file with the following name format:

```bash
nmap <remote_host>  >  <machine name>.nmap_<scanned IP>
```
