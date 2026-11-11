"""Which ERP versions this machine has: every namespace consistem/config's
config.xml expects (COMP80, DESENV83, DESENV82...), whether it exists in the
local IRIS, and the newest csw release of that version line.

Informational: a developer has only the versions they work on, so a missing
DESENV is not an error. Only the compile namespace of the current components,
COMP80, is required - it is where the tools compile.
"""

import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
from doctor import result  # noqa: E402

# doctor/iris/versions.py inside config/consisTerm: three folders up is config.
CONFIG = os.environ.get("CONSISTEM_CONFIG_DIR") or os.path.normpath(os.path.join(HERE, "..", "..", ".."))
REQUIRED = os.environ.get("DOCTOR_COMP", "COMP80")


def expected():
    """namespace -> version, from config.xml: DESENV83 -> 8.3."""
    path = os.path.join(CONFIG, "config.xml")
    if not os.path.isfile(path):
        return None
    found = {}
    for p in ET.parse(path).getroot().iter("Projeto"):
        ns = (p.get("namespace") or "").upper()
        m = re.fullmatch(r"(COMP|DESENV)(\d)(\d)", ns)
        if m:
            found[ns] = f"{m.group(2)}.{m.group(3)}"
    return found


def existing(names):
    """The namespaces of `names` that exist, asked of the local instance."""
    instance = os.environ.get("CONSISTERM_IRIS_INSTANCE")
    if not instance:
        return None, "Nenhuma instância IRIS"
    exe = None
    for root in [r"C:\InterSystems", r"C:\Program Files\InterSystems"] if os.name == "nt" else []:
        if os.path.isdir(root):
            for d in os.listdir(root):
                cand = os.path.join(root, d, "bin", "irissession.exe")
                if os.path.isfile(cand):
                    exe = cand
    args = [exe, instance, "-U", "%SYS"] if exe else ["iris", "session", instance, "-U", "%SYS"]
    code = "".join(
        f'write:##class(%SYS.Namespace).Exists("{n}") !,"<<NS>>{n}"\r\n' for n in names
    ) + "halt\r\n"
    flags = 0x08000000 if os.name == "nt" else 0
    try:
        out = subprocess.run(args, input=code, capture_output=True, text=True,
                             encoding="utf-8", errors="replace", timeout=30, creationflags=flags)
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, str(e)
    return set(re.findall(r"<<NS>>(\w+)", out.stdout)) - {"NS"}, None


def newest_csw(line):
    """Newest csw release of a line, from atualizador/csw/csw_8-2/csw_8-2-2.xml."""
    folder = os.path.join(CONFIG, "atualizador", "csw", "csw_" + line.replace(".", "-"))
    if not os.path.isdir(folder):
        return None
    versions = []
    for name in os.listdir(folder):
        m = re.fullmatch(r"csw_(\d+)-(\d+)-(\d+)(?:b(\d+))?\.xml", name)
        if m:
            a, b, c, beta = m.groups()
            versions.append(((int(a), int(b), int(c), beta is None, int(beta or 0)),
                             f"{a}.{b}.{c}" + (f"b{beta}" if beta else "")))
    return max(versions)[1] if versions else None


names = expected()
if names is None:
    result("unknown", f"config.xml não encontrado em {CONFIG}")
have, error = existing(sorted(names))
if have is None:
    result("unknown", "O IRIS não respondeu", error)

lines = []
for ns in sorted(names, key=lambda n: (n.startswith("COMP"), names[n]), reverse=True):
    release = newest_csw(names[ns]) if ns.startswith("DESENV") else None
    state = "instalado" if ns in have else "não instalado"
    lines.append(f"{ns} ({names[ns]}): {state}" + (f" - última liberação csw {release}" if release else ""))

desenvs = sorted((n for n in have if n.startswith("DESENV")), reverse=True)
summary = "Versões: " + (", ".join(f"{n[6]}.{n[7]}" for n in desenvs) if desenvs else "nenhum DESENV")
if REQUIRED.upper() not in have:
    result("error", f"{REQUIRED} não existe - é onde os componentes compilam. " + summary, lines)
result("ok", summary, lines)
