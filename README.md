This is no longer maintained since I've switched to Debian. OPNsense has turned out to be insecure, and is run by people who do not know how to maintain an operating system or have an intelligent conversation: https://github.com/opnsense/ports/issues/168

For Linux routers like Debian, some of the functionality of this dashboard is already present in [hwtop,](https://github.com/Arnavion/hwtop) namely CPU usage, temperatures and interface traffic stats. I plan to add disk usage and interface IPs to hwtop eventually. Services and firewall logs are better served through bespoke shell scripts that match what your router uses for service management and routing. For example, a systemd-using router can monitor services using `systemctl is-running`, and an nftables- / firewalld-using router can monitor firewall logs using `journalctl -ft kernel --grep='_DROP:'`.

If you have questions about this repository, open an issue at https://github.com/Arnavion/archived-repos-issues

---

The OPNsense status dashboard in your terminal instead of a web browser.


# Example

```
Version       : OPNsense 22.1-amd64
                FreeBSD 13.0-STABLE
Uptime        : 0 days 15:20:47
CPU usage     :  18.1 %
Memory usage  :  94.9 % of 16230 MiB
States table  :   0.0 % (    306 / 1623000)
MBUF usage    :   2.2 % (  21960 / 1011119)
Disk usage    :      / :   0.5 % of 187.6 GB
                  /tmp :   0.0 % of 687.0 MB
                  /var :   6.9 % of 737.9 MB
                /zroot :   0.0 % of 186.7 GB
SMART status  : ada0 S0Z4NEAC948908 PASSED
Temperatures  :           dev.cpu.0.temperature :  33.0 °C
                          dev.cpu.1.temperature :  31.0 °C
                          dev.cpu.2.temperature :  32.0 °C
                          dev.cpu.3.temperature :  30.0 °C
                hw.acpi.thermal.tz0.temperature :  27.9 °C
                hw.acpi.thermal.tz1.temperature :  29.9 °C
                                           ada0 :  33.0 °C
Interfaces    :  em0 :  26.4 Mb/s down 987.6 Kb/s up ***
                gif0 :  13.7 Kb/s down  39.6 Kb/s up ***::2
                igb0 : no carrier                    ***::1
                                                     ***::1
                                                     ***::1
                                                     192.168.1.1
                                                     192.168.5.1
                igb1 :  11.4 Kb/s down   7.7 Kb/s up 2***2::1
                                                     192.168.2.1
                igb2 :  34.2 Kb/s down  10.3 Kb/s up ***::1
                                                     192.168.3.1
                igb3 : 178.9 Kb/s down 469.7 Kb/s up ***::1
                                                     192.168.4.1
Gateways      :     HE :   97.3 ms (  12.8 ms)   0 %
                ISP_V4 :   21.0 ms (   0.9 ms)   0 %
                ISP_V6 : dpinger is not running
Services      : configd    dhcpd6     ntpd       radvd      tor       
                dhcpd      haproxy    openssh    syslog-ng  unbound   
Firewall logs : 2022-01-28T19:47:07 em0  block  6379/tcp <- 185.185.82.124
                2022-01-28T19:47:01 em0  block  5353/tcp <- 66.240.236.116
                2022-01-28T19:46:57 em0  block  2096/tcp <- 162.142.125.140
                2022-01-28T19:46:57 em0  block 42209/tcp <- 79.124.62.110
                2022-01-28T19:46:39 em0  block 22072/tcp <- 79.124.62.86
                2022-01-28T19:46:39 em0  block  7179/tcp <- 91.240.118.15
                2022-01-28T19:46:33 em0  block  8869/tcp <- 45.143.200.18
                2022-01-28T19:46:00 em0  block  5555/tcp <- 82.200.42.157
                2022-01-28T19:45:59 em0  block  5555/tcp <- 82.200.42.157
                2022-01-28T19:45:54 em0  block  5984/tcp <- 192.241.213.42
```

The output refreshes every second. It also uses colors that are not visible here.


# How to use

1. Copy config.yaml.example to `~/.config/opnsense-dashboard/config.yaml` and edit it to match your router.

1. Build and install the binary under `$PATH`, such as in `~/.local/bin`.

   ```sh
   cargo build --release
   cp -f ./target/release/opnsense-dashboard ~/.local/bin/
   ```

   `make install` will do this for you.

1. Run `opnsense-dashboard`.

   ```sh
   opnsense-dashboard
   ```

Note, the program assumes your router uses a little-endian x86_64 C ABI. If this is not the case, edit the constants in the "Router C ABI definitions" section at the top of `src/main.rs`.


# License

```
opnsense-dashboard-cli

https://github.com/Arnavion/opnsense-dashboard-cli

Copyright 2019 Arnav Singh

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
