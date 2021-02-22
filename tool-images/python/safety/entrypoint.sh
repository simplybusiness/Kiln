#!/bin/sh
START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
FILE_NAME=$(mktemp -t -p . tool-output.XXXXXXXXXX)
safety check --json | tee $FILE_NAME
END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
pip uninstall insecure-package
pip uninstall blackduck
pip uninstall django
/data-forwarder --tool-name=safety --tool-version=0.6.1 --tool-output-path="$FILE_NAME" --start-time="$START_TIME" --end-time="$END_TIME" --output-format=JSON --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"
rm $FILE_NAME
