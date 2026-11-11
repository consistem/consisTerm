"""Each csw image this machine runs is the newest released for dev.

The newest version comes from consistem/config, through gh:
atualizador/<product>/<product>_<line>/<product>_<version>.xml. A release
lists the containers built in it, and AmbienteDevLiberado=1 marks it released
for dev environments. Only the release lines in use here (8-0, 1-0...) are
read, and only their newest releases, so the check takes seconds, not minutes.

Without gh, it reads a local clone of consistem/config instead - the one this
pack lives in, when it does - and says so, since a clone is only as new as its
last pull.
"""

import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "lib"))
from doctor import DOCKER_DIR, compose_services, env_file, result  # noqa: E402

REPO = "consistem/config"
PER_LINE = 30
HERE = os.path.dirname(os.path.abspath(__file__))
# doctor/docker/images.py inside config/consisTerm: three folders up is config.
CLONE = os.environ.get("CONSISTEM_CONFIG_DIR") or os.path.normpath(
    os.path.join(HERE, "..", "..", "..")
)


def gh(path, raw=False):
    args = ["gh", "api", path]
    if raw:
        args += ["-H", "Accept: application/vnd.github.raw"]
    flags = 0x08000000 if os.name == "nt" else 0
    out = subprocess.run(
        args, capture_output=True, text=True, encoding="utf-8", check=True, creationflags=flags
    )
    return out.stdout if raw else json.loads(out.stdout)


def version_key(text):
    """8.0.46b2 / 8-0-46b2 -> sortable; a final release after its betas."""
    m = re.fullmatch(r"(\d+)[.-](\d+)[.-](\d+)(?:b(\d+))?", text or "")
    if not m:
        return None
    major, minor, patch, beta = m.groups()
    return (int(major), int(minor), int(patch), beta is None, int(beta or 0))


def local_images():
    env = env_file()
    images = {}
    for image in compose_services().values():
        m = re.match(r"\S+/(csw-[\w-]+):(\S+)", image or "")
        if m:
            name, tag = m.groups()
            images[name] = re.sub(r"\$\{(\w+)\}", lambda v: env.get(v.group(1), ""), tag)
    return images


def releases_from_gh():
    """(path under atualizador, reader) from GitHub."""
    tree = gh(f"repos/{REPO}/git/trees/HEAD:atualizador?recursive=1")["tree"]
    paths = [item["path"] for item in tree]
    return paths, lambda path: gh(f"repos/{REPO}/contents/atualizador/{path}", raw=True)


def releases_from_clone():
    root = os.path.join(CLONE, "atualizador")
    if not os.path.isdir(root):
        return None
    paths = []
    for folder, _, files in os.walk(root):
        for name in files:
            paths.append(os.path.relpath(os.path.join(folder, name), root).replace("\\", "/"))

    def read(path):
        with open(os.path.join(root, path), encoding="utf-8", errors="replace") as f:
            return f.read()

    return paths, read


def newest(wanted, lines, paths, reader):
    by_line = defaultdict(list)
    for path in paths:
        m = re.fullmatch(r"([^/]+/[^/]+)/[a-z]+_(\d+-\d+-\d+(?:b\d+)?)\.xml", path)
        key = m and version_key(m.group(2))
        if key and key[:2] in lines:
            by_line[m.group(1)].append((key, m.group(2), path))
    picked = [r for items in by_line.values() for r in sorted(items, reverse=True)[:PER_LINE]]

    def read(release):
        key, version, path = release
        try:
            root = ET.fromstring(reader(path))
        except Exception:
            return key, version, []
        if root.findtext("AmbienteDevLiberado", "0").strip() != "1":
            return key, version, []
        return key, version, [(c.text or "").strip() for c in root.iter("Container")]

    found = {}
    with ThreadPoolExecutor(8) as pool:
        for key, version, containers in pool.map(read, picked):
            for name in containers:
                if name in wanted and (name not in found or key > found[name][0]):
                    found[name] = (key, version.replace("-", "."))
    return {name: version for name, (_, version) in found.items()}


compose = os.path.join(DOCKER_DIR, "docker-compose.yml")
if not os.path.isfile(compose):
    result("skip", "Sem docker-compose.yml nesta máquina", compose)
images = local_images()
if not images:
    result("skip", "Nenhuma imagem csw no docker-compose.yml")
lines_in_use = {version_key(t)[:2] for t in images.values() if version_key(t)}
source = "GitHub"
try:
    paths, reader = releases_from_gh()
except (FileNotFoundError, subprocess.CalledProcessError) as e:
    clone = releases_from_clone()
    if clone is None:
        why = "gh não está instalado" if isinstance(e, FileNotFoundError) else "gh não conseguiu ler consistem/config (gh auth login?)"
        result("unknown", f"{why}, e não há um clone local do config", getattr(e, "stderr", None))
    paths, reader = clone
    source = f"clone local em {CLONE} (atualizado só até o último pull)"
released = newest(set(images), lines_in_use, paths, reader)

updates = {}
behind, unplaced, lines = [], [], [f"Versões liberadas lidas de: {source}"]
for name, tag in sorted(images.items()):
    latest = released.get(name)
    if latest is None or version_key(tag) is None:
        unplaced.append(name)
        lines.append(f"{name}: {tag or '(sem versão)'}, sem liberação conhecida em atualizador")
    elif version_key(tag) < version_key(latest):
        behind.append(name)
        updates[name] = latest
        lines.append(f"{name}: {tag}, a última liberada é {latest}")
    else:
        lines.append(f"{name}: {tag}, atualizada")

if behind:
    result(
        "warn",
        f"{len(behind)} de {len(images)} desatualizadas: {', '.join(behind)}",
        lines,
        updates=updates,
    )
if len(unplaced) == len(images):
    result("unknown", "Nenhuma imagem encontrada em atualizador", lines)
if source != "GitHub":
    # A clone is only as new as its last pull: "up to date" by it is not
    # something to show green.
    result(
        "warn",
        f"{len(images) - len(unplaced)} imagens atualizadas segundo o clone local - sem gh, confira com git pull",
        lines,
    )
result("ok", f"{len(images) - len(unplaced)} imagens atualizadas", lines)
