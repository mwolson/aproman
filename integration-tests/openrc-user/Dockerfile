FROM rust:1-alpine

RUN apk add --no-cache bash
WORKDIR /repo
COPY . .
RUN cargo build --release
RUN adduser -D test
RUN install -Dm755 target/release/aproman /home/test/.local/bin/aproman
USER test
CMD ["bash", "integration-tests/openrc-user/test.sh"]
