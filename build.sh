#!/bin/sh
export CARGO_HOME=/media/avinash/Data-Linux/workspace/personal/projects/pulse-rs/.cargo-home
export RUSTUP_HOME=/home/avinash/.rustup
exec cargo build -p pulse-core --offline "$@"
