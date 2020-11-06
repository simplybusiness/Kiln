ARG BASE_IMAGE=alpine:3
FROM $BASE_IMAGE


RUN apk --no-cache add ca-certificates
RUN apk --no-cache add git


WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

ARG YARN_AUDIT_VERSION 
RUN apk add yarn  
CMD ["/entrypoint.sh"]
