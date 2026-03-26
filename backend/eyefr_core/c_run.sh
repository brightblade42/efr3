#!/bin/bash

cargo build -p fr-api
RUST_LOG="info,fr_api=info,libfr=info" \
cargo run -p fr-api
