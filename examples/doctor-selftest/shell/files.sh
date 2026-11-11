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
printf 'COMPOSE_PROJECT_NAME=csw\nCSW_DESENV_VERSION=8.0.46b2\nTZ=America/Sao_Paulo\n' > "$root/a/b/c/consistem.env"
step "gravar arquivo" $?
cp "$root/a/b/c/consistem.env" "$root/consistem.env.bak"; step "copiar arquivo" $?
mkdir -p "$root/moved" && mv "$root/a/b/c/consistem.env" "$root/moved/"
[ -f "$root/moved/consistem.env" ] && [ ! -e "$root/a/b/c/consistem.env" ]; step "mover arquivo" $?
sed -i 's/^CSW_DESENV_VERSION=.*/CSW_DESENV_VERSION=8.0.46b3/' "$root/moved/consistem.env"
grep -qx 'CSW_DESENV_VERSION=8.0.46b3' "$root/moved/consistem.env" \
    && grep -qx 'TZ=America/Sao_Paulo' "$root/moved/consistem.env"
step "editar uma linha específica (sed)" $?
mv "$root/moved" "$root/renamed"; step "renomear pasta" $?
rm -rf "$root"
[ ! -e "$root" ]; step "apagar a árvore" $?

total=${#steps[@]}
status=ok; [ "$failed" -gt 0 ] && status=error
detail=$(printf '"%s",' "${steps[@]}"); detail="[${detail%,}]"
printf '{"status":"%s","summary":"%d de %d operações de arquivo","detail":%s}\n' \
    "$status" "$((total - failed))" "$total" "$detail"
