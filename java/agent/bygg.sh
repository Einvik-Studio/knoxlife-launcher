#!/usr/bin/env bash
# Bygger knoxlife-agent.jar og kjører prøvene. Samme kode gir de samme bytene, så hvem som helst kan bygge på nytt og
# sammenligne med filen launcheren laster ned.
#
#   ./bygg.sh            bygg agenten og kjør alle prøvene
#   UT=<mappe> ./bygg.sh bygg et annet sted (standard: ut/)
#
# Trenger JDK 21 eller nyere (javac, jar, java), curl og sha256sum – alt finnes i Git Bash på Windows med en JDK.
set -euo pipefail
cd "$(dirname "$0")"

UT="${UT:-ut}"
RELEASE=21
DATO="2026-01-01T00:00:00Z"
BYGG="${KNOXLIFE_BYGG:-0}"
ASM_VERSJON="$(sed -n 's/^[0-9a-f]* asm-\([0-9.]*\)\.jar .*/\1/p' libs.lock)"
SEP=":"
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) SEP=";" ;; esac
# Valgene i miljøet (for eksempel en proxy) skal ikke farge bygget eller prøvene.
unset JAVA_TOOL_OPTIONS _JAVA_OPTIONS JDK_JAVA_OPTIONS || true

sha() { sha256sum "$1" | cut -c1-64; }

rm -rf "$UT/klasser" "$UT/innhold" "$UT/pakket" "$UT/prover" "$UT/e2e"
mkdir -p "$UT/libs" "$UT/klasser" "$UT/innhold/META-INF/licenses" "$UT/pakket" "$UT/prover" "$UT/e2e"
# Windows (Git Bash): Java trenger stier som D:/…, ikke /d/….
UT="$(cd "$UT" && (pwd -W 2>/dev/null || pwd))"

echo "== bibliotekene (Maven Central, sjekket mot libs.lock)"
LIBS=()
while read -r forventet navn adresse; do
  adresse="${adresse%$'\r'}"
  [[ -z "${forventet:-}" || "$forventet" == \#* ]] && continue
  fil="$UT/libs/$navn"
  if [[ ! -f "$fil" || "$(sha "$fil")" != "$forventet" ]]; then
    curl -fsSL --retry 6 --retry-all-errors --retry-delay 1 -o "$fil.del" "$adresse"
    if [[ "$(sha "$fil.del")" != "$forventet" ]]; then
      echo "Feil SHA-256 for $navn" >&2
      exit 1
    fi
    mv "$fil.del" "$fil"
  fi
  LIBS+=("$fil")
done < libs.lock
CP="$(IFS="$SEP"; echo "${LIBS[*]}")"

echo "== kompilerer agenten"
find src -name '*.java' | LC_ALL=C sort > "$UT/kilder.txt"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -cp "$CP" -d "$UT/klasser" @"$UT/kilder.txt"

echo "== prøvene"
find prover/knoxlife -name '*.java' | LC_ALL=C sort > "$UT/prover-kilder.txt"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -cp "$CP$SEP$UT/klasser" -d "$UT/prover" @"$UT/prover-kilder.txt"
java -Dstdout.encoding=UTF-8 -cp "$CP$SEP$UT/klasser$SEP$UT/prover" knoxlife.agent.Prover

echo "== setter sammen agenten"
for lib in "${LIBS[@]}"; do
  pakke="$UT/pakket/$(basename "$lib" .jar)"
  mkdir -p "$pakke"
  (cd "$pakke" && jar xf "$lib")
done
cp "$UT/pakket/sponge-mixin-0.17.5+mixin.0.8.7/LICENSE.txt" "$UT/innhold/META-INF/licenses/sponge-mixin-LICENSE.txt"
cp "$UT/pakket/mixinextras-common-0.5.4/LICENSE_MixinExtras" "$UT/innhold/META-INF/licenses/MixinExtras-LICENSE.txt"
for pakke in "$UT"/pakket/*; do
  # Bare klassene: ikke bibliotekenes egne tjenestelister, signaturer og modulfiler, og ikke delene for andre
  # modlastere (LaunchWrapper og ModLauncher) eller verktøyene som bare brukes når mods kompileres.
  rm -rf "$pakke/META-INF" "$pakke/module-info.class" "$pakke"/LICENSE* \
    "$pakke/org/spongepowered/asm/service/mojang" "$pakke/org/spongepowered/asm/service/modlauncher" \
    "$pakke/org/spongepowered/tools" "$pakke"/org/spongepowered/asm/launch/MixinLaunchPlugin*.class \
    "$pakke"/org/spongepowered/asm/launch/MixinTransformationService*.class
  cp -R "$pakke/." "$UT/innhold/"
done
cp lisenser/asm-LICENSE.txt "$UT/innhold/META-INF/licenses/asm-LICENSE.txt"
cp ../../LICENSE "$UT/innhold/META-INF/licenses/knoxlife-LICENSE.txt"
cp -R "$UT/klasser/." "$UT/innhold/"
cp -R ressurser/. "$UT/innhold/"

# Én jar med faste tider og fast rekkefølge: samme kode gir de samme bytene.
lag_jar() { # innhold-mappe, jar-fil
  local innhold="$1" jar="$2"
  # Mixin leser ASM-versjonen fra pakken org/objectweb/asm/ i manifestet, så den må stå der.
  printf 'Premain-Class: knoxlife.agent.Agent\nCan-Redefine-Classes: false\nCan-Retransform-Classes: false\n\nName: knoxlife/agent/\nImplementation-Title: Knox Life agent\nImplementation-Version: %s\n\nName: org/objectweb/asm/\nImplementation-Title: ASM\nImplementation-Version: %s\n' "$BYGG" "$ASM_VERSJON" > "$UT/manifest.txt"
  (cd "$innhold" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) | sed "s|^\(.*\)\$|-C \"$innhold\" \"\1\"|" > "$UT/jar-filer.txt"
  rm -f "$jar"
  jar --create --file "$jar" --manifest "$UT/manifest.txt" --date="$DATO" @"$UT/jar-filer.txt"
}
lag_jar "$UT/innhold" "$UT/knoxlife-agent.jar"
echo "knoxlife-agent.jar  $(sha "$UT/knoxlife-agent.jar")  $(wc -c < "$UT/knoxlife-agent.jar") bytes"

echo "== prøven ende til ende: et etterlignet spill med en prøve-mod"
mkdir -p "$UT/e2e/spill" "$UT/e2e/mixin" "$UT/e2e/innhold"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -d "$UT/e2e/spill" prover/spill/prove/Spillet.java
(cd "$UT/e2e/spill" && jar --create --file ../projectzomboid.jar --date="$DATO" .)
javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -cp "$CP$SEP$UT/e2e/projectzomboid.jar" -d "$UT/e2e/mixin" \
  prover/mixin/knoxlife/mixin/SpilletMixin.java
cp -R "$UT/innhold/." "$UT/e2e/innhold/"
cp -R "$UT/e2e/mixin/." "$UT/e2e/innhold/"
cp prover/mixin/knoxlife.mixins.json "$UT/e2e/innhold/knoxlife.mixins.json"
lag_jar "$UT/e2e/innhold" "$UT/e2e/knoxlife-agent.jar"

uten="$(java -cp "$UT/e2e/projectzomboid.jar" prove.Spillet)"
[[ "$uten" == *"hilsen=vanlig"* && "$uten" == *"tall=1"* ]] || { echo "uten agent: $uten" >&2; exit 1; }

STEMPEL="$UT/e2e/Zomboid/KnoxLife/stempel.txt"
mkdir -p "$(dirname "$STEMPEL")"
printf 'adresse=play.example.invalid\nport=16261\nbillett=prove\n' > "$(dirname "$STEMPEL")/reise.txt"
med="$(java -XX:-EnableDynamicAgentLoading -javaagent:"$UT/e2e/knoxlife-agent.jar" -Dknoxlife.bygg=7 \
  -Dknoxlife.stempel="$STEMPEL" -cp "$UT/e2e/projectzomboid.jar" prove.Spillet)"
echo "$med" | sed 's/^/   /'
[[ "$med" == *"hilsen=endret av knoxlife"* ]] || { echo "Mixin ble ikke lagt inn" >&2; exit 1; }
[[ "$med" == *"tall=42"* ]] || { echo "MixinExtras ble ikke lagt inn" >&2; exit 1; }
[[ "$med" == *"rapport=v1|7|ok"* && "$med" != *"rapport=v1|7|ok|"* ]] || { echo "rapporten er feil" >&2; exit 1; }
grep -q '^bygg=7$' "$STEMPEL" || { echo "stempelet mangler" >&2; cat "$STEMPEL" >&2; exit 1; }
[[ ! -f "$(dirname "$STEMPEL")/reise.txt" ]] || { echo "billetten ble ikke slettet" >&2; exit 1; }

med_juks="$(java -XX:+EnableDynamicAgentLoading -javaagent:"$UT/e2e/knoxlife-agent.jar" -Dknoxlife.bygg=7 \
  -cp "$UT/e2e/projectzomboid.jar$SEP$UT/e2e/mixin" prove.Spillet)"
[[ "$med_juks" == *"dynamic agents allowed"* && "$med_juks" == *"classes in folder mixin"* ]] \
  || { echo "juksesperren så ikke juksen: $med_juks" >&2; exit 1; }

echo "== ferdig: $UT/knoxlife-agent.jar"
