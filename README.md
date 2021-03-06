The OPNsense status dashboard in your terminal instead of a web browser.


# Example

```
Version       : OPNsense 21.1.2-amd64
                FreeBSD 12.1-RELEASE-p13-HBSD

Uptime        : 0 days 04:14:47

CPU usage     :  10.4 %
Memory usage  :   2.2 % of 32617 MiB
States table  :   0.0 % (    136 / 3261000)
MBUF usage    :   0.8 % (  15800 / 2034500)
Disk usage    :    / :   0.8 % of 247.0 GB
                /var :   0.1 % of  32.5 GB
                /tmp :   0.0 % of  32.4 GB
SMART status  : ada0 S0Z4NEAC948908 PASSED

Temperatures  :           dev.cpu.0.temperature :  33.0 °C
                          dev.cpu.1.temperature :  32.0 °C
                          dev.cpu.2.temperature :  32.0 °C
                          dev.cpu.3.temperature :  28.0 °C
                hw.acpi.thermal.tz0.temperature :  27.9 °C
                hw.acpi.thermal.tz1.temperature :  29.9 °C
                                           ada0 :  37.0 °C

Interfaces    :  em0 :   1.8 Kb/s down   1.2 Kb/s up ***.***.***.***
                gif0 :   1.3 Kb/s down   0    b/s up ****:****:****:****::2
                igb0 : no carrier                    ****:****:****:1::1
                                                     ****:****:****:5::1
                                                     192.168.1.1
                igb1 :   0    b/s down   0    b/s up ****:****:****:2::1
                                                     192.168.2.1
                igb2 :   0    b/s down   0    b/s up ****:****:****:3::1
                                                     192.168.3.1
                igb3 : 152.5 Kb/s down 370.6 Kb/s up ****:****:****:4::1
                                                     192.168.4.1
Gateways      :     HE :   24.1 ms (   0.1 ms)   0 %
                ISP_V4 :   20.7 ms (   1.4 ms)   0 %
                ISP_V6 : dpinger is not running

Services      : configd    dhcpd6     ntpd       radvd      syslogd
                dhcpd      haproxy    openssh    syslog-ng  unbound

Firewall logs : Mar  6 20:48:41 em0  block 31823/tcp <- 89.248.174.2
                Mar  6 20:48:41 em0  block 42615/udp <- 209.126.38.54
                Mar  6 20:48:40 em0  block  8999/tcp <- 154.29.131.161
                Mar  6 20:48:39 em0  block 31823/tcp <- 89.248.174.2
                Mar  6 20:48:38 em0  block 31823/tcp <- 89.248.174.2
                Mar  6 20:48:34 em0  block  8999/tcp <- 185.159.158.58
                Mar  6 20:48:32 em0  block  8999/tcp <- 154.29.131.161
                Mar  6 20:48:29 em0  block  9091/tcp <- 45.146.165.148
                Mar  6 20:48:28 em0  block  8999/tcp <- 154.29.131.161
                Mar  6 20:48:25 em0  block  8999/tcp <- 154.29.131.161
```

The output refreshes every second. It also uses colors that are not visible here.


# How to use

1. Copy config.yaml.example to `~/.config/opnsense-dashboard/config.yaml` and edit it to match your router.

1. Build and install the binary under `$PATH`, such as in `~/bin`.

   ```sh
   cargo build --release
   cp -f ./target/release/opnsense-dashboard ~/bin/
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
