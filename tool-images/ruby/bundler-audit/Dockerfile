FROM ruby:2-alpine
RUN apk --no-cache add git
WORKDIR /code

COPY entrypoint.sh /entrypoint.sh
RUN ["chmod", "+x", "/entrypoint.sh"]
COPY data-forwarder /data-forwarder
RUN ["chmod", "+x", "/data-forwarder"]

ARG BUNDLER_AUDIT_VERSION
RUN gem install bundler-audit -v $BUNDLER_AUDIT_VERSION
RUN bundler audit update
CMD ["/entrypoint.sh"]
