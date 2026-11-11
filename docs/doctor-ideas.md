# Doctor: checks and fixes still to write

Ideas collected while the engine was built, for the Consistem checks file. What exists today is in
`examples/doctor/`: the runtimes, Docker running, the `csw_default` network, the containers, the
image versions, and the COMP and DESENV namespaces. See [doctor.md](doctor.md) for the format.

## Where things are

- **Released image versions**: `consistem/config` › `atualizador/<product>/<product>_<line>/<product>_<version>.xml`.
  The file name is the version; `<Containers>` lists the images built in that release;
  `<AmbienteDevLiberado>1</AmbienteDevLiberado>` marks it released for dev environments.
  `examples/doctor/docker/images.py` reads it through `gh api`.
- **What this machine runs**: `C:\consistem\lib\docker\docker-compose.yml` (tags typed in for
  `csw-desenv`, `csw-sandbox`; `${CSW_*_VERSION}` for the rest) and `consistem.env` (the versions,
  plus tokens and the IRIS password - never print it).
- **IRIS and the web gateway** have a compose file of their own, `docker-compose-intersystems.yaml`,
  written by `lib/bin/intersystems`.
- **The tools** in `C:\consistem\lib\bin` are bash: `consistem start|stop|restart|recreate|update|pull|logs|env`,
  `consistem_update <image> <version>` (only application, nginx, tag, external), `consistem_install`
  (interactive; asks for the IRIS password), `intersystems` (iris and webgateway).

## Fixes to write

- **Update an image** (`docker.images`). Today's manual step is editing the tag in the compose
  file and running `consistem recreate <service>`. The fix:
  - for an image with a `${CSW_X_VERSION}`: `consistem env -e csw-x=<version>` then
    `consistem recreate x`;
  - for a tag typed into `docker-compose.yml` (`csw-desenv`, `csw-sandbox`): rewrite that one
    `image:` line, keeping a `.bak`, then `consistem recreate <service>`.
  - Careful: `consistem env` rewrites `consistem.env` from a fixed list of variables - try it on a
    copy first, and check nothing outside the list is lost.
  - `<after>`: `docker.images` and `docker.containers`.
- **IRIS / web gateway image** (`csw-iris`, `csw-webgateway`): `intersystems recreate`, which writes
  its compose file again with the version.
- **Missing network / stopped containers**: `consistem start` (already in the example).
- **COMP or DESENV missing**: an ObjectScript entry reference that creates the namespace and its
  mappings, or the team's existing setup routine. Needs the expected mappings written down first.
- **From scratch**: `consistem_install` in a tab (it asks for the password there), then the fixes
  above in order. Python and Git Bash are prerequisites and are not installed by the Doctor.

## Checks to write

- **csw-sandbox's released version**: it is not listed in any `atualizador` XML. Find where its
  releases are recorded, or leave it out of `images.py`.
- **ERP product version**: the CSW version installed in DESENV against the latest released
  (`atualizador/csw/...`) and against the images' version line.
- **COMP and DESENV are the right ones**: on the machine this was tried on the compile namespace is
  `COMP80`, not `COMP`. Check by version (`COMP80`, `DESENV80`), and that their routine and global
  databases map where the team expects.
- **Versioned bases** (`config/bases_versionadas.xml`): the local LDB's version against the
  namespace's.
- **Containers healthy, not only running**: `docker inspect --format {{.State.Health.Status}}`
  where a healthcheck exists (sandbox has one).
- **Ports free / answering**: 443 and 80 (desenv), 8090 (sandbox), 30196 (calcserpro), 1972 and
  52773 (IRIS).
- **Certificates**: `lib/keystores/fullchain.pem` present and not about to expire.
- **gh signed in** with access to `consistem/config` - `images.py` depends on it; today it says
  `unknown` when it is not.
- **Disk space** on the drive holding `C:\consistem` and the Docker data.
- **Git hooks** installed from `config/git-hooks-v2` in the ERP repositories.

## Engine ideas, not built

- A dot on the **Doctor** entry of the sidebar in the overall status colour, as Clawd has.
- **Fix all**: walk the list from the top, run each failing check's fix, verify, go on - the
  "install from scratch" button.
- Run independent checks in parallel; today they run one after another (about 13 s for the example,
  most of it `images.py`).
- Verify a fix by itself when its tab's program exits, rather than by the Verify button.
- Pass the `DOCTOR_*` variables to a fix tab's environment too, not only through `{{name}}`.
- Ship the Consistem pack beside the program (`org-doctor.xml`) from the `config` repository, the
  way `consisTerm/macros.xml` is.

## Done since

- `iris/versions.py`: the COMP/DESENV namespaces `config.xml` expects, which exist here, and the
  newest csw release of each line. Error only when COMP80 is missing.
- `fixes/update_images.py`: the image update - writes the new tag (consistem.env or the compose
  line, with `.bak`), then `consistem recreate <service>`, asking first.

Next candidates found in `config`: the versioned bases (`bases_versionadas.xml`) against what is
checked out locally; the installed csw version inside each DESENV against `atualizador/csw`
(needs where DESENV records its version - not found yet); the Adapcon namespaces of `config.xml`.
