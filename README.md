# Pool Monitor

> :warning: **VERY MUCH WORK IN PROGRESS**: ambitions: plenty, finished work: more

## prerequisites

### rustup

https://rustup.rs/

### espup

```shell
cargo install espup
espup install
```

### environment (constants for the build)

Set these environments, or put them in a `.env` file,
which will be source directly using [`direnv`](https://direnv.net/):

```shell
export ESP32_WIFI_SSID=YOURWIFI
export ESP32_WIFI_PASS=YOURPASSWORD
```

## running

This command: `cargo run --release` will:
- download and build dependencies 
- build the code
- use `espflash` to flash the software onto a device
- boot the device and connect the logger to the console




