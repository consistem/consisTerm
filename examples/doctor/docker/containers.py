"""Every service of lib/docker/docker-compose.yml has a running container."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "lib"))
from doctor import DOCKER_DIR, compose_services, env_file, result, run  # noqa: E402

compose = os.path.join(DOCKER_DIR, "docker-compose.yml")
if not os.path.isfile(compose):
    result("skip", "Sem docker-compose.yml nesta máquina", compose)

services = compose_services(compose)
project = env_file().get("COMPOSE_PROJECT_NAME", "csw")
code, out = run(
    [
        "docker", "ps",
        "--filter", f"label=com.docker.compose.project={project}",
        "--format", '{{.Label "com.docker.compose.service"}}',
    ]
)
if code is None or code != 0:
    result("unknown", "docker ps falhou", out)

running = set(out.split())
stopped = [s for s in services if s not in running]
lines = [f"{s}: {'rodando' if s in running else 'parado'}" for s in services]
if stopped:
    result(
        "error",
        f"{len(stopped)} de {len(services)} parados: {', '.join(stopped)}",
        lines,
    )
result("ok", f"Os {len(services)} serviços estão rodando", lines)
