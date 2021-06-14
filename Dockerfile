# vim:set ft=dockerfile:

FROM debian:buster-slim

RUN set -eux; apt-get update; apt-get install -y --no-install-recommends libpq5 libc6 libgcc1

COPY static_files/ /usr/local/share/channel_builder/static_files/
COPY target/debug/channel_builder /usr/local/bin

WORKDIR /usr/local/share/channel_builder
ENTRYPOINT ["channel_builder"]

EXPOSE 3031
