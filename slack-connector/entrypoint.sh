#!/bin/sh
cp /tls/* /usr/local/share/ca-certificates
/usr/sbin/update-ca-certificates
su-exec kilnapp:kilngroup /app/slack-connector
