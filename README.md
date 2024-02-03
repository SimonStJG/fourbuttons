Four Buttons
============

A machine with four light up buttons in a pretty box.  It reminds you to do certain tasks, like water the plants, on a set schedule.  Also an excuse to learn Rust.

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

## TODO

* Store Schedule in the DB
* Connect the events to the schedule table
* Add bi-weekly reminders
* Add reminder which checks if the previous activity was actioned in a 
  reasonable amount of time and then email me if it wasn't
