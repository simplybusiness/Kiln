FROM ruby:2-alpine
RUN apk --no-cache add git
WORKDIR /code

ENV BUNDLER_AUDIT_VERSION 0.6.1
RUN gem install bundler-audit -v $BUNDLER_AUDIT_VERSION
RUN bundler audit update
CMD ["bundle", "audit", "check", "--update"]
