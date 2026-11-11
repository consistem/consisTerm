"""The csw_default network the Consistem containers are attached to."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "lib"))
from doctor import result, run  # noqa: E402

code, out = run(["docker", "network", "inspect", "csw_default", "--format", "{{.Name}}"])
if code is None:
    result("unknown", "docker não respondeu", out)
if code != 0:
    result("error", "A rede csw_default não existe", out)
result("ok", "csw_default existe")
