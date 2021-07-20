#!/bin/sh
FILE_NAME=$(mktemp -t -p . tool-output.XXXXXXXXXX)
START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
YARN_VERSION=$(yarn audit --version)
yarn audit --json --no-progress | tee $FILE_NAME 
END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
/data-forwarder --tool-name=yarn-audit --tool-version="$YARN_VERSION" --tool-output-path="$FILE_NAME" --start-time="$START_TIME" --end-time="$END_TIME" --output-format=JSON --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"
rm $FILE_NAME
