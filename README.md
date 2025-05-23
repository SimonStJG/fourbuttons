Four Buttons
============

A machine with four light up buttons in a pretty box.  It reminds you to do certain tasks, like water the plants, on a set schedule.  Also an excuse to learn Rust.

## Pre-requisites

* docker or [podman](https://podman.io/docs/installation).
* `cargo install cross` - for cross compilation

## Usage

* Run locally with `USE_FAKE_RPI=1 RUST_LOG=debug cargo run`.
* Release with `./release.sh`.
* Autoformat code with `cargo fmt`.

## Pretty pictures

![Picture of the finished machine](images/complete.png)

## Circuits

A circuit diagram showing a single LED and button, in reality there are 4 of each.
![Circuit Diagram](images/circuit-2.png)

The stripboarded elements (transistor & resistor):
![Stripboard diagram](images/circuit-1.png)

## First time Raspberry Pi Configuration

Automatically connect to the wifi on startup:
1. `nmcli device wifi connect "<SSID>" password "<password>"`
2. `nmcli connection up "<SSID>"`
3. `nmcli connection mod "<SSID>" connection.autoconnect yes`
(To see if the wifi is connected: `nmcli connection show`)

Manually reserved 192.168.0.94 on the router.

You can also do this manually by creating an nmconnection file in 
/etc/NetworkManager/system-connections (e.g. copy the previous one
and replace the SSID, password, etc).

## TODO

Some things I could do?  But who has the time.
* Tidy up my half-baked DIY actor framework!
 * Actually test it, write some tests
 * Cleanly shutdown the other actors when one dies
 * Handle sigterm and shutdown cleanly (this looks cute for the blocking syscalls? https://mazzo.li/posts/stopping-linux-threads.html#homegrown-thread-cancellation)
* Test the core notification logic now it isn't all tied up with threads.
* Store scheduler state in DB.
* Create a proper config class / don't store all the secrets in separate text files
