#!/bin/sh
FILE_NAME=$(mktemp -t -p . tool-output.XXXXXXXXXX)
REQ_FILE_NAME=requirements.txt
if test -f "$REQ_FILE_NAME"; then 
    START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
    safety check --json -r requirements.txt | tee $FILE_NAME
    END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
    /data-forwarder --tool-name=safety --tool-version=1.10.3 --tool-output-path="$FILE_NAME" --start-time="$START_TIME" --end-time="$END_TIME" --output-format=JSON --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"
    rm $FILE_NAME
fi
