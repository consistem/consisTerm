"""Self-test, Python: a git update - a repository gets a commit, and a clone
of it is brought up to date with `git pull`. All in a temporary folder: no
real repository is touched."""

import json
import os
import shutil
import subprocess
import tempfile

if shutil.which("git") is None:
    print(json.dumps({"status": "skip", "summary": "git não está instalado"}))
    raise SystemExit(0)

FLAGS = 0x08000000 if os.name == "nt" else 0
# Who commits, without touching the user's own git configuration.
IDENTITY = ["-c", "user.name=Doctor", "-c", "user.email=doctor@example.invalid", "-c", "init.defaultBranch=main"]


def git(*args, cwd):
    done = subprocess.run(["git", *IDENTITY, *args], cwd=cwd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", creationflags=FLAGS)
    if done.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {done.stderr.strip()}")
    return done.stdout.strip()


root = tempfile.mkdtemp(prefix="consisterm-selftest-git-")
steps = []
try:
    origin, work, mine = (os.path.join(root, n) for n in ("origin.git", "work", "mine"))
    git("init", "--bare", origin, cwd=root)
    git("clone", origin, work, cwd=root)
    open(os.path.join(work, "versao.txt"), "w").write("8.0.46b2\n")
    git("add", ".", cwd=work)
    git("commit", "-m", "primeira", cwd=work)
    git("push", "origin", "HEAD:main", cwd=work)
    git("clone", origin, mine, cwd=root)
    steps.append("ok: clonar")

    open(os.path.join(work, "versao.txt"), "w").write("8.0.46b3\n")
    git("commit", "-am", "nova versão", cwd=work)
    git("push", "origin", "HEAD:main", cwd=work)
    before = git("rev-parse", "HEAD", cwd=mine)
    git("pull", "--ff-only", cwd=mine)
    after = git("rev-parse", "HEAD", cwd=mine)
    updated = open(os.path.join(mine, "versao.txt")).read().strip() == "8.0.46b3"
    steps.append(f"{'ok' if before != after and updated else 'FALHOU'}: git pull trouxe a versão nova")
    status = "ok" if before != after and updated else "error"
    summary = "git pull atualizou o clone" if status == "ok" else "git pull não atualizou"
except RuntimeError as e:
    status, summary = "error", str(e)
finally:
    # Git marks its objects read-only on Windows; rmtree needs them writable.
    def writable(func, path, _):
        os.chmod(path, 0o700)
        func(path)
    shutil.rmtree(root, onerror=writable)

print(json.dumps({"status": status, "summary": summary, "detail": steps}, ensure_ascii=False))
