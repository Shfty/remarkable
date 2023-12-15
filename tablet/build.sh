#!/bin/sh

cargo build --release
ssh root@remarkable 'killall -q -9 tray; killall -q -9 wave'
scp target/armv7-unknown-linux-gnueabihf/release/wave root@remarkable:~
scp target/armv7-unknown-linux-gnueabihf/release/tray root@remarkable:~
ssh root@remarkable './wave'
