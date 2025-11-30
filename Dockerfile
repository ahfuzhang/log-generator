FROM alpine:3.20

WORKDIR /home/appuser
RUN adduser -D appuser

# Expect a prebuilt, MUSL-linked static binary.
COPY target/x86_64-unknown-linux-musl/release/log-generator /usr/local/bin/log-generator

USER appuser
ENTRYPOINT ["log-generator"]
CMD ["--sleep_ms", "0", "--batch_bytes", "64k", "--output", "stdout"]
