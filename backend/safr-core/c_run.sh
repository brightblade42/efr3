#!/bin/bash

cargo build -p fr-api
TPASS_URL="https://devsys01.tpassvms.com/TpassPVService/" \
TPASS_USER="admin" \
TPASS_PWD="njbs1968" \
SAFR_DB_ADDR="100.79.241.8" \
SAFR_DB_PORT="5432" \
SAFR_DB_USER="admin" \
SAFR_DB_PWD="admin" \
SAFR_DB_NAME="identity" \
SAFR_DB_SSLMODE="disable" \
PV_PROC_URL="http://100.79.241.8:50051" \
PV_IDENT_URL="http://100.79.241.8:5656" \
FR_BACKEND="paravision" \
FR_REMOTE="tpass" \
FRAPI_PORT="3000" \
RUST_LOG="info,fr_api=info,libfr=info,libpv=info,libtpass=info" \
cargo run -p fr-api
