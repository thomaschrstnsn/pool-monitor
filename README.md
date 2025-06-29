# Pool Monitor

> :warning: **VERY MUCH WORK IN PROGRESS**: ambitions: plenty

## prerequisites

### rustup

https://rustup.rs/

### espup

```shell
cargo install espup
espup install
```

### espflash

```shell
cargo install espflash
```


### environment (constants for the build)

Set these environments, or put them in a `.env` file,
which will be source directly using [`direnv`](https://direnv.net/):

```shell
export ESP32_WIFI_SSID=YOURWIFI
export ESP32_WIFI_PASS=YOURPASSWORD
export POST_ENDPOINT_IP=IP_OF_HTTP_ENDPOINT     # IPv4 of server accepting HTTP POST with JSON payload of temperature readings
export POST_ENDPOINT_PORT=PORT_OF_HTTP_ENDPOINT # TCP port of the server
```

## running

This command: `cargo run --release` will:
- download and build dependencies
- build the code
- use `espflash` to flash the software onto a device
- boot the device and connect the logger to the console

## known good environment

```shell
❯ cargo --version
cargo 1.77.0-nightly (3fe68eabf 2024-02-29)

❯ espup --version
espup 0.12.0

❯ espflash --version
espflash 3.3.0
```

```
espup install --toolchain-version 1.77.0.0
```

### nix tmpdir issue

For this issue when using nixpkgs rustup:

```
   Compiling pool-monitor v0.1.0 (/Users/thomas/src/pool-monitor)
error: couldn't create a temp dir: No such file or directory (os error 2) at path "/private/tmp/nix-shell-16093-0/rustcMZcKhH"
```

```shell
TMPDIR=/tmp cargo build --release
```

