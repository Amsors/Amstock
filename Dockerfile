FROM node:24-bookworm-slim AS frontend-build
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.97-bookworm AS backend-build
WORKDIR /build/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN cargo build --release --locked

FROM python:3.13-slim-bookworm AS python-deps
RUN python -m venv /opt/venv
COPY printer/requirements.txt /tmp/requirements.txt
RUN /opt/venv/bin/pip install --no-cache-dir -r /tmp/requirements.txt

FROM python:3.13-slim-bookworm AS backend
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates fonts-dejavu-core fonts-noto-cjk libusb-1.0-0 sqlite3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 amstock \
    && useradd --uid 1000 --gid 1000 --no-create-home --shell /usr/sbin/nologin amstock

COPY --from=python-deps /opt/venv /opt/venv
COPY --from=backend-build /build/backend/target/release/amstock-backend /usr/local/bin/amstock-backend
COPY printer /app/printer
COPY deploy/amstock-export /usr/local/bin/amstock-export
COPY deploy/amstock-import /usr/local/bin/amstock-import
RUN chmod 0755 /usr/local/bin/amstock-export /usr/local/bin/amstock-import \
    && mkdir -p /data/images /data/label-previews /backups \
    && chown -R 1000:1000 /data /backups

ENV PATH="/opt/venv/bin:${PATH}" \
    AMSTOCK_DATABASE_URL="sqlite:///data/amstock.db" \
    AMSTOCK_IMAGE_DIR="/data/images" \
    AMSTOCK_BIND="0.0.0.0:3000" \
    AMSTOCK_PRINTER_PYTHON="/opt/venv/bin/python" \
    AMSTOCK_PRINTER_SCRIPT="/app/printer/amstock_printer.py" \
    AMSTOCK_LABEL_PREVIEW_DIR="/data/label-previews" \
    AMSTOCK_OPEN_LABEL_PREVIEW="false" \
    RUST_LOG="amstock_backend=info,tower_http=info"
WORKDIR /app
USER 1000:1000
EXPOSE 3000
CMD ["amstock-backend"]

FROM caddy:2.10-alpine AS web
COPY Caddyfile /etc/caddy/Caddyfile
COPY --from=frontend-build /build/frontend/dist /srv
