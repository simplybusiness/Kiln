#!/bin/sh
cp /tls/* /usr/local/share/ca-certificates
/usr/sbin/update-ca-certificates
gosu kilnapp:kilngroup /app/report-parser
