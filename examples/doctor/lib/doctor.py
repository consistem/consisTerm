"""Helpers for a Doctor check written in Python.

A check prints one line of JSON as its last word; `result` does that and
exits, so a check reads as a list of `if ...: result(...)`.

    import os, sys
    sys.path.insert(0, os.path.join(os.environ["CONSISTERM_DOCTOR_DIR"], "lib"))
    from doctor import result, run, CONSISTEM_HOME
"""

import json
import os
import shutil
import subprocess
import sys

CONSISTEM_HOME = os.environ.get(
    "CONSISTEM_HOME", "c:/consistem" if os.name == "nt" else "/consistem"
)
DOCKER_DIR = os.path.join(CONSISTEM_HOME, "lib", "docker")


def result(status, summary="", detail=None, **extra):
    """Prints the answer and ends the check. `detail` may be text or a list
    of lines; `extra` fields ride along for a fix to read - the Doctor shows
    only status, summary and detail."""
    answer = {"status": status, "summary": summary}
    if detail:
        answer["detail"] = detail
    answer.update(extra)
    print(json.dumps(answer, ensure_ascii=False))
    sys.exit(0)


def run(args, timeout=30):
    """Runs a command; returns (exit code, output) - output being stdout and
    stderr together. A program that is not installed comes back as code None."""
    if shutil.which(args[0]) is None:
        return None, f"{args[0]}: not found"
    flags = 0x08000000 if os.name == "nt" else 0  # no console window
    try:
        done = subprocess.run(
            args,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            creationflags=flags,
        )
    except subprocess.TimeoutExpired:
        return None, f"{args[0]}: no answer in {timeout} s"
    return done.returncode, (done.stdout + done.stderr).strip()


def env_file(path=None):
    """The variables of consistem.env, as a dict. Never print the values:
    the file holds tokens and passwords."""
    path = path or os.path.join(DOCKER_DIR, "consistem.env")
    values = {}
    if os.path.isfile(path):
        for line in open(path, encoding="utf-8"):
            line = line.rstrip("\n")
            if "=" in line and not line.startswith("#"):
                key, value = line.split("=", 1)
                values[key.strip()] = value
    return values


def compose_services(path=None):
    """Each service of docker-compose.yml, with the image line it has."""
    import re

    path = path or os.path.join(DOCKER_DIR, "docker-compose.yml")
    services, current, inside = {}, None, False
    for line in open(path, encoding="utf-8"):
        if re.match(r"^\S", line):
            inside = line.startswith("services:")
            current = None
            continue
        if not inside:
            continue
        name = re.match(r"^  ([\w.-]+):\s*$", line)
        if name:
            current = name.group(1)
            services[current] = None
            continue
        image = re.match(r"^\s+image:\s*(\S+)", line)
        if image and current:
            services[current] = image.group(1)
    return services
