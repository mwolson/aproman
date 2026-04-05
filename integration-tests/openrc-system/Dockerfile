FROM alpine:3.21

RUN apk add --no-cache bash openrc python3 procps

COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

COPY README.md pyproject.toml /build/
COPY aproman/ /build/aproman/
COPY systemd/ /build/systemd/
COPY openrc-system/ /build/openrc-system/
COPY openrc-user/ /build/openrc-user/
RUN uv pip install --system --break-system-packages /build/ && \
    if ! [ -f /usr/local/bin/aproman ]; then \
        ln -sf "$(command -v aproman)" /usr/local/bin/aproman; \
    fi

COPY integration-tests/shared/fake-pactl /usr/local/bin/pactl
COPY integration-tests/shared/fake-dbus-monitor /usr/local/bin/dbus-monitor
RUN chmod +x /usr/local/bin/pactl /usr/local/bin/dbus-monitor

COPY integration-tests/openrc-system/test.sh /opt/test.sh
RUN chmod +x /opt/test.sh

CMD ["/opt/test.sh"]
