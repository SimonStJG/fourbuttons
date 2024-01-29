#!/bin/bash

set -euxo pipefail

cross build --target arm-unknown-linux-gnueabihf --release
ssh simon@192.168.0.94 "sudo systemctl stop fourbuttons"
scp target/arm-unknown-linux-gnueabihf/release/fourbuttons fourbuttons.service 99-fourbuttons.rules simon@192.168.0.94:/home/simon/
ssh simon@192.168.0.94 "
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

echo 'To watch the logs, run:'
echo '  ssh simon@192.168.0.94 "journalctl -u fourbuttons -f"'