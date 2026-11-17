#!/usr/bin/env bash
# Bygger knoxlife-agent.jar og kjører prøvene. Samme kode gir de samme bytene, så hvem som helst kan bygge på nytt og
# sammenligne med filen launcheren laster ned.
#
#   ./bygg.sh            bygg agenten og kjør alle prøvene
#   UT=<mappe> ./bygg.sh bygg et annet sted (standard: ut/)
#   SPILLETS_MAPPE=<mappe> ./bygg.sh   med Java-modene: mods/ kompileres mot spillets egne jar-filer (mappen med
#                                      projectzomboid.jar, i den dedikerte serveren `java/`), og porten starter
#                                      agenten mot spillet. Trenger samme Java som spillet (25). Uten mappen bygges
#                                      agenten uten Java-modene – bare til prøvene, aldri til spillerne.
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

rm -rf "$UT/klasser" "$UT/innhold" "$UT/pakket" "$UT/prover" "$UT/e2e" "$UT/mods" "$UT/oppsett"
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

SPILL_CP=""
MODS=()
if [[ -n "${SPILLETS_MAPPE:-}" ]]; then
  [[ -f "$SPILLETS_MAPPE/projectzomboid.jar" ]] || { echo "Fant ikke projectzomboid.jar i $SPILLETS_MAPPE" >&2; exit 1; }
  SPILL_CP="$(find "$SPILLETS_MAPPE" -maxdepth 1 -name '*.jar' | LC_ALL=C sort | tr '\n' "$SEP")"
  echo "== kompilerer Java-modene mot spillet"
  rm -rf "$UT/mods" && mkdir -p "$UT/mods"
  find mods -name '*.java' | LC_ALL=C sort > "$UT/mods-kilder.txt"
  javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -cp "$CP$SEP$UT/klasser$SEP$SPILL_CP" -d "$UT/mods" \
    @"$UT/mods-kilder.txt"
  # Hver klasse i knoxlife/mixin/ som slutter på Mixin, er én Java-mod.
  while read -r k; do MODS+=("$k"); done < <(cd "$UT/mods/knoxlife/mixin" && find . -maxdepth 1 -name '*Mixin.class' \
    | sed 's|^\./||; s|\.class$||' | LC_ALL=C sort)
  echo "   ${#MODS[@]} Java-mods: ${MODS[*]}"
fi
# Oppsettet til Mixin: listen over Java-modene, og hver innføring må treffe (defaultRequire), ellers stopper agenten.
liste=""
for k in ${MODS[@]+"${MODS[@]}"}; do liste+="${liste:+, }\"$k\""; done
mkdir -p "$UT/oppsett"
printf '{\n  "required": true,\n  "minVersion": "0.8",\n  "package": "knoxlife.mixin",\n  "compatibilityLevel": "JAVA_21",\n  "mixins": [%s],\n  "injectors": { "defaultRequire": 1 }\n}\n' "$liste" > "$UT/oppsett/knoxlife.mixins.json"

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
cp "$UT/oppsett/knoxlife.mixins.json" "$UT/innhold/"
if [[ ${#MODS[@]} -gt 0 ]]; then cp -R "$UT/mods/." "$UT/innhold/"; fi

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

mkdir -p "$UT/porten"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -proc:none -cp "$CP$SEP$UT/klasser" -d "$UT/porten" \
  porten/knoxlife/porten/Porten.java

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
  -Dknoxlife.stempel="~/Zomboid/KnoxLife/stempel.txt" -Duser.home="$UT/e2e" -cp "$UT/e2e/projectzomboid.jar" prove.Spillet)"
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

echo "== porten stopper når spillet har endret seg"
mkdir -p "$UT/e2e/endret"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -d "$UT/e2e/endret" prover/spill-endret/prove/Spillet.java
(cd "$UT/e2e/endret" && jar --create --file ../endret.jar --date="$DATO" .)
if java -XX:-EnableDynamicAgentLoading -javaagent:"$UT/e2e/knoxlife-agent.jar" \
    -cp "$UT/e2e/endret.jar$SEP$UT/porten" knoxlife.porten.Porten "$UT/e2e/endret.jar" > "$UT/e2e/porten.txt" 2>&1; then
  echo "porten slapp gjennom et spill der prøve-moden ikke traff" >&2
  cat "$UT/e2e/porten.txt" >&2
  exit 1
fi
grep -q "could not find any targets matching 'hilsen'" "$UT/e2e/porten.txt" \
  || { echo "porten stoppet, men av feil grunn:" >&2; cat "$UT/e2e/porten.txt" >&2; exit 1; }
echo "   stoppet som den skal: prøve-moden fant ikke metoden «hilsen»"

echo "== porten slipper gjennom når alt treffer, og stopper når klassen er borte"
ut_porten="$(java -XX:-EnableDynamicAgentLoading -javaagent:"$UT/e2e/knoxlife-agent.jar" \
  -cp "$UT/e2e/projectzomboid.jar$SEP$UT/porten" knoxlife.porten.Porten "$UT/e2e/projectzomboid.jar" 2>&1)" \
  || { echo "porten stoppet et spill der alt traff:" >&2; echo "$ut_porten" >&2; exit 1; }
[[ "$ut_porten" == *"1 klasser i spillet skal ha Java-mods"* ]] || { echo "$ut_porten" >&2; exit 1; }
mkdir -p "$UT/e2e/borte"
javac --release "$RELEASE" -encoding UTF-8 -nowarn -d "$UT/e2e/borte" prover/spill-borte/prove/Annet.java
(cd "$UT/e2e/borte" && jar --create --file ../borte.jar --date="$DATO" .)
if java -XX:-EnableDynamicAgentLoading -javaagent:"$UT/e2e/knoxlife-agent.jar" \
    -cp "$UT/e2e/borte.jar$SEP$UT/porten" knoxlife.porten.Porten "$UT/e2e/borte.jar" > "$UT/e2e/borte.txt" 2>&1; then
  echo "porten slapp gjennom et spill der klassen prøve-moden skal inn i, er borte" >&2
  cat "$UT/e2e/borte.txt" >&2
  exit 1
fi
grep -q "prove.Spillet got no Java mod" "$UT/e2e/borte.txt" \
  || { echo "porten stoppet, men av feil grunn:" >&2; cat "$UT/e2e/borte.txt" >&2; exit 1; }
echo "   slapp gjennom når alt traff, og stoppet når klassen var borte"

if [[ -n "${SPILLETS_MAPPE:-}" ]]; then
  echo "== porten: agenten mot spillets egen jar ($SPILLETS_MAPPE)"
  java -XX:-EnableDynamicAgentLoading -javaagent:"$UT/knoxlife-agent.jar" -Dknoxlife.bygg="$BYGG" \
    -cp "$SPILL_CP$UT/porten" knoxlife.porten.Porten "$SPILLETS_MAPPE/projectzomboid.jar"
fi

echo "== ferdig: $UT/knoxlife-agent.jar"
