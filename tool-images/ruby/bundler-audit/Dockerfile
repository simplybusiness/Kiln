FROM ruby:2-alpine
RUN apk --no-cache add git
WORKDIR /code

ARG BUNDLER_AUDIT_VERSION
RUN gem install bundler-audit -v $BUNDLER_AUDIT_VERSION
RUN bundler audit update
CMD ["bundle", "audit", "check", "--update"]
