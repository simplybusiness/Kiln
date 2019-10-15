FROM alpine:3.10
WORKDIR /app
RUN addgroup -S kilngroup && adduser -S kilnapp -G kilngroup 
USER kilnapp
COPY --chown=kilnapp:kilngroup target/x86_64-unknown-linux-musl/release/data-collector /app
ENTRYPOINT ["/app/data-collector"]
