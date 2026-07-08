#!/usr/bin/env bash
#
# Deploys Steins Reader as a systemd service on Linux.
# Run this script from the root of the cloned repo (the folder
# containing frontend/ and backend/), e.g.:
#   git clone <repo-url> steins-reader
#   cd steins-reader
#   sudo ./deploy-steins-reader.sh

set -euo pipefail

REPO="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
INSTALL_DIR="/opt/steins-reader"
DATA_DIR="/var/lib/steins-reader"
SERVICE_NAME="steins-reader"
UNIT_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

step() {
    echo -e "\n\033[1;36m==> $1\033[0m"
}

if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root (use sudo)." >&2
    exit 1
fi

if [[ ! -d "$REPO/frontend" || ! -d "$REPO/backend" ]]; then
    echo "Expected frontend/ and backend/ next to this script (repo root: $REPO)." >&2
    exit 1
fi

# 1. Stop and remove existing service if present (idempotency)
if systemctl list-unit-files | grep -q "^${SERVICE_NAME}.service"; then
    step "Existing service '${SERVICE_NAME}' found. Stopping and removing it."
    systemctl stop "${SERVICE_NAME}" || true
    systemctl disable "${SERVICE_NAME}" || true
    rm -f "$UNIT_FILE"
    systemctl daemon-reload
fi

# 2. Build frontend
step "Building frontend"
pushd "$REPO/frontend" > /dev/null
if [[ -f package-lock.json ]]; then
    npm ci
else
    npm install
fi
npm run build
popd > /dev/null

rm -rf "$REPO/backend/dist"
cp -r "$REPO/frontend/dist" "$REPO/backend/dist"

# 3. Build backend release binary
step "Building backend (release)"
pushd "$REPO/backend" > /dev/null
cargo build --release
popd > /dev/null

# 4. Stage install and data directories
step "Staging files"
mkdir -p "$INSTALL_DIR" "$DATA_DIR" "$DATA_DIR/logs"
cp "$REPO/backend/target/release/steins-reader" "$INSTALL_DIR/steins-reader"
chmod +x "$INSTALL_DIR/steins-reader"
rm -rf "$INSTALL_DIR/dist"
cp -r "$REPO/backend/dist" "$INSTALL_DIR/dist"

# 5. Write service config
step "Writing config.json"
cat > "$DATA_DIR/config.json" <<EOF
{
  "port": 1337,
  "db_path": "${DATA_DIR}/steins_reader.db",
  "frontend_dir": "${INSTALL_DIR}/dist",
  "log_dir": "${DATA_DIR}/logs"
}
EOF

# 6. Register systemd service and enable crash recovery
step "Registering systemd service"
cat > "$UNIT_FILE" <<EOF
[Unit]
Description=Steins Reader
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/steins-reader --config ${DATA_DIR}/config.json
Restart=on-failure
RestartSec=5
StartLimitIntervalSec=86400
StartLimitBurst=3
User=root

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "${SERVICE_NAME}"

step "Starting service"
systemctl start "${SERVICE_NAME}"
systemctl is-active --quiet "${SERVICE_NAME}"

step "Done. Checking status"
systemctl status "${SERVICE_NAME}" --no-pager
