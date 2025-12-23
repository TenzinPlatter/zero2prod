#!/usr/bin/env bash

export APP_ENVIRONMENT="PRODUCTION"

LOGFILE="logs/$(date +%Y-%m-%d_%H-%M-%S).log"
mkdir -p logs
/usr/local/bin/zero2prod -c /configuration.yaml | tee "$LOGFILE" | bunyan --color
