#!/bin/bash
set -euo pipefail 

function exit () {
        gpioset 0 23=0
        gpioset 0 24=0
        gpioset 0 27=0
        gpioset 0 22=0
}

trap "exit" EXIT

while :; do
        gpioset 0 23=1
        sleep 1
        gpioset 0 23=0
        gpioset 0 24=1
        sleep 1
        gpioset 0 24=0
        gpioset 0 27=1
        sleep 1
        gpioset 0 27=0
        gpioset 0 22=1
        sleep 1
        gpioset 0 22=0
done