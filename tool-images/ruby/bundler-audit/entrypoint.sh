#!/bin/sh
START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
bundle audit check --update | tee tool-output.txt 
END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")

/data-forwarder --tool-name=bundler-audit --tool-version=0.6.1 --tool-output-path=tool-output.txt --start-time="$START_TIME" --end-time="$END_TIME" --output-format=PlainText --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"

rm tool-output.txt
