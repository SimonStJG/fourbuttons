#!/bin/bash

set -euxo pipefail

TARGET=192.168.1.94

cargo fmt --check
cargo clippy
cargo test
cross build --target arm-unknown-linux-gnueabihf --release
ssh simon@${TARGET} "sudo systemctl stop fourbuttons"
scp target/arm-unknown-linux-gnueabihf/release/fourbuttons \
    fourbuttons.service \
    99-fourbuttons.rules \
    mailgun-apikey \
    to-address \
    selftest/selftest.sh \
    simon@${TARGET}:/home/simon/
ssh simon@${TARGET} "
    set -euxo pipefail

    sudo mv fourbuttons.service /etc/systemd/system/fourbuttons.service
    sudo chown root:root /etc/systemd/system/fourbuttons.service
    sudo chmod 644 /etc/systemd/system/fourbuttons.service
    
    sudo mv 99-fourbuttons.rules /etc/udev/rules.d/99-fourbuttons.rules
    sudo chown root:root /etc/udev/rules.d/99-fourbuttons.rules
    sudo chmod 644 /etc/udev/rules.d/99-fourbuttons.rules

    sudo systemctl daemon-reload
    sudo systemctl start fourbuttons
    sudo systemctl enable fourbuttons
"

echo 'Watching logs...'

ssh simon@${TARGET} "journalctl -u fourbuttons -f"