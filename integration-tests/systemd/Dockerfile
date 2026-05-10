FROM rust:1-bookworm

WORKDIR /repo
COPY . .
RUN cargo build --release
RUN useradd -m test
RUN install -Dm755 target/release/aproman /home/test/.local/bin/aproman
USER test
CMD ["bash", "integration-tests/systemd/test.sh"]
