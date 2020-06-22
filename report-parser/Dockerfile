ARG BASE_IMAGE=alpine:3.10
FROM $BASE_IMAGE
WORKDIR /app
RUN apk add --no-cache ca-certificates

ENV GOSU_VERSION=1.12
RUN set -eux; \
	\
	apk add --no-cache --virtual .gosu-deps gnupg curl \
    && gpg --keyserver hkps://keys.openpgp.org --recv-keys B42F6819007F00F88E364FD4036A9C25BF357DD4 \
    && curl -o /usr/local/bin/gosu -SL "https://github.com/tianon/gosu/releases/download/${GOSU_VERSION}/gosu-amd64" \
    && curl -o /usr/local/bin/gosu.asc -SL "https://github.com/tianon/gosu/releases/download/${GOSU_VERSION}/gosu-amd64.asc" \
    && gpg --verify /usr/local/bin/gosu.asc \
    && rm /usr/local/bin/gosu.asc \
    && rm -rf /root/.gnupg/ \
    && apk del --no-network .gosu-deps \
    && chmod +x /usr/local/bin/gosu \
    # Verify that the binary works
    && gosu nobody true

COPY entrypoint.sh /app
RUN chmod +x /app/entrypoint.sh
RUN addgroup -S kilngroup && adduser -S kilnapp -G kilngroup 
COPY --chown=kilnapp:kilngroup target/x86_64-unknown-linux-musl/release/report-parser /app
ENTRYPOINT ["/bin/sh", "-c", "/app/entrypoint.sh"]
