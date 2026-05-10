FROM rust:1-alpine

RUN apk add --no-cache bash
WORKDIR /repo
COPY . .
RUN cargo build --release
CMD ["bash", "integration-tests/openrc-system/test.sh"]
