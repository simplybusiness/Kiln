ARG BASE_IMAGE=ruby:2-alpine
FROM $BASE_IMAGE
RUN apk --no-cache add ca-certificates
RUN apk --no-cache add git
RUN gem install bundler -v "~>1.0" && \
    gem install bundler -v "~>2.0"
WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

RUN gem install bundler-audit -v 0.6.1
RUN bundler audit update
CMD ["/entrypoint.sh"]
