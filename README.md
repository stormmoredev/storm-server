# Storm Server

A simple HTTP/HTTPS server with support for load balancing, virtual hosts, resource browsing, and PHP.  
It allows multiple domains to run on a single port (default: **80** for HTTP, **443** for HTTPS).  
Storm Server can be used for development as a terminal process or as a Windows service.

---

## Status
Storm Server is under active development.  
Currently it supports basic HTTP/HTTPS features and load balancing.  
At the moment it runs only on **Windows**, but support for **Linux** and **macOS** is planned.

---

## Features
- HTTP/HTTPS support
- Multiple domains on a single port (virtual hosts)
- Load balancing
- Static file serving with optional directory browsing
- PHP support
- Windows service mode

---

## Build
```bash
# Build and run the server
cargo run --release --bin stormsrv

# Build and run as a service
cargo run --release --bin stormsrvsvc
````

---

## Running Storm Server

```bash
stormsrver [options]
```

### Options

* `-p 8080` – port to listen on
* `-d /path/to/directory` – directory to serve

When started without parameters, Storm Server listens on **port 80** and serves files from the current working directory.

---

## Running as a Windows Service

### Create a service

```bash
sc create stormsrvsvc binPath= "C:\path\to\stormsrvsvc.exe"
```

### Start the service

```bash
sc start stormsrvsvc
```

### Delete the service

```bash
sc delete stormsrvsvc
```

---

## Service Configuration

When the service is started, it creates the directory `C:\stormsrv` with two subdirectories:

* `logs` – server logs
* `conf` – configuration files (`*.conf`)

After modifying or adding a configuration file, restart the service.

### Example configuration (`yourname.conf`)

```ini
server.dir = C:\your_dir
server.port = 443
server.browsing_enabled = yes
server.domain = yourdomain.com

php.enabled = true
php.index = index.php
;php.port = 9000
;php.socket = /run/php/php8.3-fpm.sock

logs.enabled = yes
logs.min_level = debug
logs.dir = D:\storm-server-www\logs\stormphp.com

https.enabled = true
https.public_key = D:\storm-server-www\certs\stormphp.com.pem
https.private_key = D:\storm-server-www\certs\stormphp.com-key.pem

load_balancer.enabled = no
load_balancer.servers = 127.0.0.1:100
load_balancer.servers = 127.0.0.1:101
load_balancer.servers = 127.0.0.1:102
```

---

## Virtual Hosts

If you want to configure multiple domains, make sure that each of them uses the same HTTPS settings.
Each domain should have its own SSL certificates.
Note: You cannot mix HTTP and HTTPS domains on the same port.

---

## Creating SSL Certificates

The easiest way to generate a local SSL certificate is with [`mkcert`](https://github.com/FiloSottile/mkcert).

### Install local Certificate Authority

```bash
mkcert -install
```

### Create a certificate

```bash
mkcert yourdomain
```

This will generate both a public and a private key.

---