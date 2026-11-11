#!/usr/bin/env bash
# Self-test, shell: a git update in a temporary folder - a repository gets a
# commit, and a clone of it is brought up to date with `git pull`.
if ! command -v git >/dev/null 2>&1; then
    echo '{"status":"skip","summary":"git não está instalado"}'
    exit 0
fi
g() { git -c user.name=Doctor -c user.email=doctor@example.invalid -c init.defaultBranch=main "$@" >/dev/null 2>&1; }

root=$(mktemp -d "${TMPDIR:-/tmp}/consisterm-selftest-git-XXXXXX")
trap 'rm -rf "$root"' EXIT
g init --bare "$root/origin.git" \
    && g clone "$root/origin.git" "$root/work" \
    && echo 8.0.46b2 > "$root/work/versao.txt" \
    && g -C "$root/work" add . && g -C "$root/work" commit -m primeira \
    && g -C "$root/work" push origin HEAD:main \
    && g clone "$root/origin.git" "$root/mine" \
    || { echo '{"status":"error","summary":"não foi possível preparar o repositório"}'; exit 0; }

echo 8.0.46b3 > "$root/work/versao.txt"
g -C "$root/work" commit -am "nova versão" && g -C "$root/work" push origin HEAD:main
before=$(git -C "$root/mine" rev-parse HEAD)
g -C "$root/mine" pull --ff-only
after=$(git -C "$root/mine" rev-parse HEAD)
if [ "$before" != "$after" ] && [ "$(cat "$root/mine/versao.txt")" = 8.0.46b3 ]; then
    echo '{"status":"ok","summary":"git pull atualizou o clone"}'
else
    echo '{"status":"error","summary":"git pull não atualizou"}'
fi
