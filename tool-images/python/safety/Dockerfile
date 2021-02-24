ARG BASE_IMAGE=python:3.10-rc-alpine
FROM $BASE_IMAGE
RUN apk --no-cache add ca-certificates
RUN apk --no-cache add git
WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

RUN pip install safety=='1.10.3'

CMD ["/entrypoint.sh"]
