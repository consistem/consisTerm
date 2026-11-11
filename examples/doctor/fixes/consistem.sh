#!/usr/bin/env bash
# A fix: runs the Consistem Docker tool (lib/bin/consistem) with the arguments
# it was given - `start`, `recreate desenv` - in the Doctor's fix tab, in view.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) home="${CONSISTEM_HOME:-/c/consistem}" ;;
    *) home="${CONSISTEM_HOME:-/consistem}" ;;
esac
"$home/lib/bin/consistem" "$@"
status=$?
echo
echo "Concluído (código $status). Volte ao Diagnóstico e clique em Conferir."
read -r -p "Enter fecha esta aba... " _
exit $status
