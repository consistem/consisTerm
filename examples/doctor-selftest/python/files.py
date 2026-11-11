"""Self-test, Python: folders and files - create a tree, write, copy, move,
edit one string in a file, read it back, and remove it all."""

import json
import os
import re
import shutil
import tempfile

steps = []


def step(name, ok):
    steps.append(f"{'ok' if ok else 'FALHOU'}: {name}")
    return ok


root = tempfile.mkdtemp(prefix="consisterm-selftest-py-")
try:
    deep = os.path.join(root, "a", "b", "c")
    os.makedirs(deep)
    step("criar pastas aninhadas", os.path.isdir(deep))

    env = os.path.join(deep, "consistem.env")
    with open(env, "w", encoding="utf-8", newline="\n") as f:
        f.write("COMPOSE_PROJECT_NAME=csw\nCSW_DESENV_VERSION=8.0.46b2\nTZ=America/Sao_Paulo\n")
    step("gravar arquivo", os.path.getsize(env) > 0)

    copy = os.path.join(root, "consistem.env.bak")
    shutil.copyfile(env, copy)
    step("copiar arquivo", os.path.isfile(copy))

    moved = os.path.join(root, "moved", "consistem.env")
    os.makedirs(os.path.dirname(moved))
    shutil.move(env, moved)
    step("mover arquivo", os.path.isfile(moved) and not os.path.exists(env))

    text = open(moved, encoding="utf-8").read()
    edited, n = re.subn(r"^CSW_DESENV_VERSION=.*$", "CSW_DESENV_VERSION=8.0.46b3", text, flags=re.M)
    with open(moved, "w", encoding="utf-8", newline="\n") as f:
        f.write(edited)
    after = open(moved, encoding="utf-8").read()
    step(
        "editar uma linha específica",
        n == 1 and "CSW_DESENV_VERSION=8.0.46b3" in after and "TZ=America/Sao_Paulo" in after,
    )

    os.rename(os.path.join(root, "moved"), os.path.join(root, "renamed"))
    step("renomear pasta", os.path.isdir(os.path.join(root, "renamed")))
finally:
    shutil.rmtree(root, ignore_errors=True)
step("apagar a árvore", not os.path.exists(root))

failed = [s for s in steps if s.startswith("FALHOU")]
print(json.dumps({
    "status": "error" if failed else "ok",
    "summary": f"{len(steps) - len(failed)} de {len(steps)} operações de arquivo",
    "detail": steps,
}, ensure_ascii=False))
