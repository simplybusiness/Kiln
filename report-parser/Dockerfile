FROM alpine:3.10
WORKDIR /app
RUN apk add --no-cache ca-certificates su-exec
COPY entrypoint.sh /app
RUN chmod +x /app/entrypoint.sh
RUN addgroup -S kilngroup && adduser -S kilnapp -G kilngroup 
COPY --chown=kilnapp:kilngroup target/x86_64-unknown-linux-musl/release/report-parser /app
ENTRYPOINT ["/bin/sh", "-c", "/app/entrypoint.sh"]
