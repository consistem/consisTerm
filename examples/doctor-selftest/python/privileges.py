"""Self-test, Python: whether the check runs with elevated privileges.

The Doctor never elevates a check: it runs as the user who opened the app.
A fix that needs an administrator asks for one itself, in its tab - see
powershell/elevated_fix.ps1. So this check is information, not a fault."""

import ctypes
import json
import os

if os.name == "nt":
    admin = bool(ctypes.windll.shell32.IsUserAnAdmin())
else:
    admin = os.geteuid() == 0
user = os.environ.get("USERNAME") or os.environ.get("USER") or "?"
print(json.dumps({
    "status": "ok",
    "summary": "Rodando como administrador" if admin else "Rodando sem elevação (o normal)",
    "detail": f"usuário: {user}",
}, ensure_ascii=False))
