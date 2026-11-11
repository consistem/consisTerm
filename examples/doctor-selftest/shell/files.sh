#!/usr/bin/env bash
# Self-test, shell: folders and files - create, write, copy, move, edit one
# line with sed, rename, remove. Git Bash on Windows, bash elsewhere.
steps=()
failed=0
step() {
    if [ "$2" = 0 ]; then steps+=("ok: $1"); else steps+=("FALHOU: $1"); failed=$((failed + 1)); fi
}

root=$(mktemp -d "${TMPDIR:-/tmp}/consisterm-selftest-sh-XXXXXX")
mkdir -p "$root/a/b/c"; step "criar pastas aninhadas" $?
printf 'APP_NAME=example\nAPP_VERSION=8.0.46b2\nTZ=America/Sao_Paulo\n' > "$root/a/b/c/settings.env"
step "gravar arquivo" $?
cp "$root/a/b/c/settings.env" "$root/settings.env.bak"; step "copiar arquivo" $?
mkdir -p "$root/moved" && mv "$root/a/b/c/settings.env" "$root/moved/"
[ -f "$root/moved/settings.env" ] && [ ! -e "$root/a/b/c/settings.env" ]; step "mover arquivo" $?
sed -i 's/^APP_VERSION=.*/APP_VERSION=8.0.46b3/' "$root/moved/settings.env"
grep -qx 'APP_VERSION=8.0.46b3' "$root/moved/settings.env" \
    && grep -qx 'TZ=America/Sao_Paulo' "$root/moved/settings.env"
step "editar uma linha específica (sed)" $?
mv "$root/moved" "$root/renamed"; step "renomear pasta" $?
rm -rf "$root"
[ ! -e "$root" ]; step "apagar a árvore" $?

total=${#steps[@]}
status=ok; [ "$failed" -gt 0 ] && status=error
detail=$(printf '"%s",' "${steps[@]}"); detail="[${detail%,}]"
printf '{"status":"%s","summary":"%d de %d operações de arquivo","detail":%s}\n' \
    "$status" "$((total - failed))" "$total" "$detail"
