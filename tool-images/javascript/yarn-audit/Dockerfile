ARG BASE_IMAGE=node:lts-alpine
FROM $BASE_IMAGE
RUN apk --no-cache add ca-certificates
RUN apk --no-cache add git
WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

CMD ["/entrypoint.sh"]
