#!/bin/sh
START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
FILE_NAME=$(mktemp -t -p . tool-output.XXXXXXXXXX)
if [[ ! -z "$OFFLINE" && $OFFLINE=="true" ]]; then
    bundle audit check | tee $FILE_NAME
else
    bundle audit check --update | tee $FILE_NAME
fi
END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")

/data-forwarder --tool-name=bundler-audit --tool-version=0.6.1 --tool-output-path="$FILE_NAME" --start-time="$START_TIME" --end-time="$END_TIME" --output-format=Plaintext --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"

