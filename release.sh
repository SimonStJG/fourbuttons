#!/bin/bash

set -euxo pipefail

cross build --target arm-unknown-linux-gnueabihf --release
ssh simon@192.168.0.94 "sudo systemctl stop fourbuttons"
scp target/arm-unknown-linux-gnueabihf/release/fourbuttons fourbuttons.service simon@192.168.0.94:/home/simon/
ssh simon@192.168.0.94 "
    sudo mv fourbuttons.service /etc/systemd/system/fourbuttons.service
    sudo systemctl daemon-reload
    sudo systemctl start fourbuttons
    sudo systemctl enable fourbuttons
"

echo 'To watch the logs, run:'
echo '  ssh simon@192.168.0.94 "journalctl -u fourbuttons -f"'