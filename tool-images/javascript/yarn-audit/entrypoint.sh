#!/bin/sh
START_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")
FILE_NAME=$(mktemp -t -p . tool-output.XXXXXXXXXX)
if [[ ! -z "$OFFLINE" && $OFFLINE=="true" ]]; then
	echo 'Error cannot run Yarn audit offline'
	rm $FILE_NAME	
	exit 1	
else
   	yarn audit
	yarn audit --json > $FILE_NAME 
fi
END_TIME=$(date -I'seconds' | sed "s/\(.*\)\(.\{2\}\)$/\1:\2/")

/data-forwarder --tool-name=yarn-audit --tool-version=1.22.4 --tool-output-path="$FILE_NAME" --start-time="$START_TIME" --end-time="$END_TIME" --output-format=JSON --scan-env="$SCAN_ENV" --app-name="$APP_NAME" --endpoint-url="$DATA_COLLECTOR_URL"

