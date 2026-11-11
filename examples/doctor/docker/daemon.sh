#!/usr/bin/env bash
# Docker installed and its daemon answering - Linux and macOS.
if ! command -v docker >/dev/null 2>&1; then
    echo '{"status":"error","summary":"docker não está no PATH"}'
    exit 0
fi
if ! version=$(docker version --format '{{.Server.Version}}' 2>/dev/null); then
    echo '{"status":"error","summary":"O serviço do Docker não responde"}'
    exit 0
fi
printf '{"status":"ok","summary":"Docker %s"}\n' "$version"
