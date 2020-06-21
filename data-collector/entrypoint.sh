#!/bin/sh
if [[ -d /tls ]]; then
    if find /tls/ -mindepth 1 | read; then
        cp /tls/* /usr/local/share/ca-certificates
        /usr/sbin/update-ca-certificates
        cp /etc/ssl/certs/ca-certificates.crt /etc/ssl/cert.pem
    fi
fi
gosu kilnapp:kilngroup /app/data-collector
