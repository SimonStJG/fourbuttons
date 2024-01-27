Four Buttons
============

* Run locally with `USE_FAKE_RPI=1 RUST_LOG=debug cargo run`.
* Release with `./release.sh`.
* Autoformat code with `cargo fmt`.

### Notes

To automatically connect to the wifi on startup:

1. `nmcli device wifi connect "<SSID>" password "<password>"`
2. `nmcli connection up "<SSID>"`
3. `nmcli connection mod "<SSID>" connection.autoconnect yes`

To see if the wifi is connected:
1. `nmcli connection show`

I also manually reserved 192.168.0.94 on the modem.
