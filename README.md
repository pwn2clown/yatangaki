# Yatangaki

Yatangaki is a MITM proxy for HTTP designed for web security testing.

The only purpose of this project is to waste my spare time by implementing this garbage. Use it at your own risks.

## Features

- HTTP/HTTPS interception (no websocket yet)
- Native GUI (no bloated javascript or embebbed browser)
- SQLite database for network logs

## Install prequisites

MSRV: 1.80

##  System dependencies

Fedora/RHEL:

```
$ dnf install sqlite-devel sqlite.x86_64 chromium nss-tools -y
```
