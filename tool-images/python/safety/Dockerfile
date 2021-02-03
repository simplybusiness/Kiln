ARG BASE_IMAGE=python:3.10-rc-alpine
FROM $BASE_IMAGE
RUN apk --no-cache add ca-certificates
RUN apk --no-cache add git
WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

RUN apk add --no-cache --virtual .pynacl_deps build-base python3-dev libffi-dev openssl-dev
RUN pip install safety
RUN pip install insecure-package
RUN pip install 'blackduck==0.0.50'
RUN pip install distro
RUn pip install django=='1.11.22'

CMD ["/entrypoint.sh"]
