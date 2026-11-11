"""A fix: brings every csw image that is behind to the newest dev release.

What a developer does by hand today, in view, asking first:
1. asks the images check which images are behind and what to;
2. writes the new tag - in consistem.env where the compose file reads a
   ${CSW_X_VERSION}, otherwise in the compose file's own image: line - keeping
   a .bak of each file it touches;
3. runs `consistem recreate <service>` for each, which pulls the image.
"""

import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
from doctor import CONSISTEM_HOME, DOCKER_DIR, compose_services  # noqa: E402

COMPOSE = os.path.join(DOCKER_DIR, "docker-compose.yml")
ENV = os.path.join(DOCKER_DIR, "consistem.env")


def behind():
    # UTF-8 both ways: a fix tab is not given PYTHONUTF8 as a check is, and
    # without it the child prints its accents in the Windows codepage.
    env = dict(os.environ, PYTHONUTF8="1", PYTHONIOENCODING="utf-8")
    out = subprocess.run([sys.executable, os.path.join(HERE, "..", "docker", "images.py")],
                         capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    if out.returncode != 0 or not out.stdout:
        raise SystemExit("A verificação das imagens falhou: " + (out.stderr or "").strip())
    for line in reversed(out.stdout.splitlines()):
        if line.startswith("{"):
            return json.loads(line).get("updates") or {}
    return {}


def backup(path):
    shutil.copyfile(path, path + ".bak")


def set_version(image, version):
    """Writes `version` where the compose file takes `image`'s tag from."""
    text = open(COMPOSE, encoding="utf-8").read()
    m = re.search(rf"(image:\s*\S+/{re.escape(image)}:)(\S+)", text)
    if not m:
        raise SystemExit(f"{image} não está em {COMPOSE}")
    var = re.fullmatch(r"\$\{(\w+)\}", m.group(2))
    if var:
        env = open(ENV, encoding="utf-8").read()
        new, n = re.subn(rf"^{var.group(1)}=.*$", f"{var.group(1)}={version}", env, flags=re.M)
        if n == 0:
            new = env.rstrip("\n") + f"\n{var.group(1)}={version}\n"
        backup(ENV)
        open(ENV, "w", encoding="utf-8", newline="\n").write(new)
        return f"consistem.env: {var.group(1)}={version}"
    backup(COMPOSE)
    open(COMPOSE, "w", encoding="utf-8", newline="").write(text[: m.start(2)] + version + text[m.end(2):])
    return f"docker-compose.yml: {image}:{version}"


def bash():
    if os.name != "nt":
        return "bash"
    for base in [os.environ.get("ProgramFiles", ""), os.environ.get("ProgramFiles(x86)", "")]:
        path = os.path.join(base, "Git", "bin", "bash.exe")
        if os.path.isfile(path):
            return path
    raise SystemExit("Git Bash não encontrado")


print("Procurando imagens desatualizadas...")
updates = behind()
if not updates:
    print("Nenhuma imagem para atualizar.")
    input("Enter fecha esta aba... ")
    sys.exit(0)

services = {img.split("/")[-1].split(":")[0]: svc for svc, img in compose_services().items() if img}
for image, version in updates.items():
    print(f"  {image} -> {version} (serviço {services.get(image, '?')})")
if input("\nAtualizar e recriar esses containers? [s/N] ").strip().lower() not in ("s", "sim", "y"):
    sys.exit(1)

tool = os.path.join(CONSISTEM_HOME, "lib", "bin", "consistem").replace("\\", "/")
status = 0
for image, version in updates.items():
    print("\n" + set_version(image, version))
    service = services.get(image)
    if service:
        status |= subprocess.run([bash(), tool, "recreate", service]).returncode

print(f"\nConcluído (código {status}). Volte ao Diagnóstico e clique em Conferir.")
print("Os arquivos anteriores ficaram como .bak em", DOCKER_DIR)
input("Enter fecha esta aba... ")
sys.exit(status)
