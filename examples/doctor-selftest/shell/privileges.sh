#!/usr/bin/env bash
# Self-test, shell: whether the check runs elevated. On Windows, `net session`
# only succeeds for an administrator; elsewhere, uid 0 is root.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) net session >/dev/null 2>&1 && admin=1 || admin=0 ;;
    *) [ "$(id -u)" = 0 ] && admin=1 || admin=0 ;;
esac
if [ "$admin" = 1 ]; then
    echo '{"status":"ok","summary":"Rodando como administrador"}'
else
    echo '{"status":"ok","summary":"Rodando sem elevação (o normal)"}'
fi
