#!/usr/bin/env bash
# Bygger en utgitt patch på nytt hjemme og sammenligner den byte for byte med den GitHub publiserte (launcher.md 3.11
# punkt 3 i det private repoet, «Build it yourself» i README.md).
#
#   utgivelse/bygg-pa-nytt.sh <utgave> <mappe med projectzomboid.jar>
#
#   <utgave>   navnet på releasen (patch-12), eller en mappe der bygget.txt og knoxlife-agent.jar fra releasen ligger
#   <mappe>    mappen med spillets projectzomboid.jar og de andre jar-filene: spillets mappe, eller java/ i den dedikerte
#              serveren (Steam-app 380870, gratis med SteamCMD uten innlogging)
#
# Koden hentes fra commiten i bygget.txt med git archive, så det du selv har endret i repoet, er ikke med. Trenger git,
# curl, sha256sum og en JDK (samme versjon som i bygget.txt gir de samme bytene) – alt finnes i Git Bash på Windows.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi
UTGAVE="$1"
SPILLET="$(cd "$2" && pwd)"
ROT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="Einvik-Studio/knoxlife-launcher"
MIDL="$(mktemp -d)"
trap 'rm -rf "$MIDL"' EXIT

sha() { sha256sum "$1" | cut -c1-64; }
verdi() { sed -n "s/^$1=//p" "$UTGIVELSE/bygget.txt" | tr -d '\r'; }

if [[ -d "$UTGAVE" ]]; then
  UTGIVELSE="$(cd "$UTGAVE" && pwd)"
else
  [[ "$UTGAVE" =~ ^patch-[0-9]+$ ]] || { echo "Utgaven heter patch-<nummer>, for eksempel patch-12." >&2; exit 2; }
  UTGIVELSE="$MIDL/utgivelse"
  mkdir -p "$UTGIVELSE"
  for f in bygget.txt knoxlife-agent.jar; do
    curl -fsSL --retry 3 -o "$UTGIVELSE/$f" "https://github.com/$REPO/releases/download/$UTGAVE/$f"
  done
fi
for f in bygget.txt knoxlife-agent.jar; do
  [[ -f "$UTGIVELSE/$f" ]] || { echo "Mangler $f i $UTGIVELSE" >&2; exit 1; }
done

BYGG="$(verdi bygg)"
COMMIT="$(verdi commit)"
[[ "$BYGG" =~ ^[0-9]+$ && "$COMMIT" =~ ^[0-9a-f]{40}$ ]] || { echo "bygget.txt kan ikke leses" >&2; exit 1; }
echo "Patch $BYGG, commit $COMMIT, spillet $(verdi spillet_versjon)"

# Spillet må være det patchen ble bygget mot.
[[ -f "$SPILLET/projectzomboid.jar" ]] || { echo "Fant ikke projectzomboid.jar i $SPILLET" >&2; exit 1; }
if [[ "$(sha "$SPILLET/projectzomboid.jar")" != "$(verdi spillet_sha256)" ]]; then
  echo "projectzomboid.jar i $SPILLET er en annen versjon av spillet enn patchen er bygget mot." >&2
  echo "Ventet SHA-256 $(verdi spillet_sha256)." >&2
  exit 1
fi

# Samme JDK gir de samme bytene; en annen versjon kan gi andre.
HER="$(java -XshowSettings:properties -version 2>&1 | sed -n 's/^ *java\.runtime\.version = //p' | tr -d '\r')"
if [[ "$HER" != "$(verdi java)" ]]; then
  echo "Advarsel: patchen ble bygget med Java $(verdi java) ($(verdi java_leverandor)), her er $HER – bytene kan bli andre."
fi

echo "== koden fra commiten"
if ! git -C "$ROT" cat-file -e "$COMMIT^{commit}" 2>/dev/null; then
  git -C "$ROT" fetch --quiet origin "$COMMIT" || { echo "Fant ikke commiten $COMMIT – kjør git fetch først." >&2; exit 1; }
fi
mkdir -p "$MIDL/kilde"
git -C "$ROT" archive "$COMMIT" | tar -x -C "$MIDL/kilde"

echo "== bygger (java/agent/bygg.sh)"
KNOXLIFE_BYGG="$BYGG" SPILLETS_MAPPE="$SPILLET" UT="$MIDL/ut" "$MIDL/kilde/java/agent/bygg.sh" > "$MIDL/bygg.txt" 2>&1 || {
  tail -n 40 "$MIDL/bygg.txt" >&2
  echo "Bygget stoppet (hele utskriften over)." >&2
  exit 1
}

UTGITT="$(sha "$UTGIVELSE/knoxlife-agent.jar")"
SELV="$(sha "$MIDL/ut/knoxlife-agent.jar")"
echo "utgitt:       $UTGITT"
echo "bygget her:   $SELV"
if [[ "$UTGITT" == "$SELV" && "$SELV" == "$(verdi agent_sha256)" ]]; then
  echo "Like byte for byte: patch $BYGG er bygget fra commiten $COMMIT."
else
  echo "IKKE like." >&2
  exit 1
fi
