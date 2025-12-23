#!/usr/bin/env bash

if [[ -n "$1" ]]; then
    formatter="$1"
else
    formatter="bunyan -o long"
fi

release_flag=""

if [[ -n "$RUST_RELEASE" ]]; then
    release_flag="--release"
fi

source .env
export APP_ENVIRONMENT="PRODUCTION"

if [[ "$formatter" == "-" ]]; then
    # allow user to specify no formatter
    RUST_LOG=info cargo run "$release_flag"
else
    RUST_LOG=info cargo run "$release_flag" | $formatter
fi
