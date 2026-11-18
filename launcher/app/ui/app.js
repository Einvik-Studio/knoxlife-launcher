// Knox Life-launcheren: skjermen. Alt arbeidet gjøres i Rust (src/main.rs); her vises det, og knappene kaller kommandoene.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const vinduet = window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);

// Det spilleren ser for hvert steg i «Play».
const STEGENE = {
  rydder: () => "Getting ready…",
  henter_oppdateringen: () => "Checking for updates…",
  laster_ned: (s) => `Downloading ${s.nr} of ${s.av}…`,
  legger_inn: () => "Preparing the game…",
  starter_steam: () => "Starting Project Zomboid…",
  venter: () => "Waiting for the game…",
  lastet: () => "Playing",
  ikke_lastet: () => "Playing without Knox Life",
  setter_tilbake: () => "Putting the game files back…",
};
const OPPDATERT =
  "Project Zomboid has updated. Knox Life is getting ready for it – usually within a few hours. See #announcements on Discord.";
const IKKE_LASTET =
  'The Knox Life patch did not load. Press "Copy report" and send it to staff on Discord.';

let iGang = false;
let klar = false;

function settStatus(tekst, prikk, detaljer) {
  $("statustekst").textContent = tekst;
  $("prikk").className = "prikk" + (prikk ? " " + prikk : "");
  if (detaljer !== undefined) $("detaljlinje").textContent = detaljer;
}

function visMelding(tekst, { rapport = false, ok = false } = {}) {
  $("meldingstekst").textContent = tekst;
  $("melding-rapport").hidden = !rapport;
  $("melding").classList.toggle("ok", ok);
  $("melding").hidden = false;
}

function skjulMelding() {
  $("melding").hidden = true;
}

function visNyheter(nyheter) {
  const liste = $("nyhetsliste");
  liste.replaceChildren();
  if (!nyheter || nyheter.length === 0) {
    const tom = document.createElement("li");
    tom.className = "kort tom";
    tom.textContent = "No news right now.";
    liste.append(tom);
    return;
  }
  for (const n of nyheter.slice(0, 3)) {
    const kort = document.createElement("li");
    kort.className = "kort";
    const dato = document.createElement("div");
    dato.className = "dato";
    dato.textContent = n.dato;
    const tittel = document.createElement("div");
    tittel.className = "overskrift";
    tittel.textContent = n.tittel;
    kort.append(dato, tittel);
    if (n.tekst) {
      const tekst = document.createElement("div");
      tekst.className = "brodtekst";
      tekst.textContent = n.tekst;
      kort.append(tekst);
    }
    liste.append(kort);
  }
}

function oppdaterPlay() {
  const play = $("play");
  play.disabled = iGang || !klar;
  if (!iGang) {
    play.textContent = "PLAY";
    play.classList.remove("i-gang");
  }
}

let steamNavn = null;
function linje2(deler) {
  if (steamNavn) deler.push(`Steam: ${steamNavn}`);
  return deler.join(" · ");
}

async function hentStatus() {
  settStatus("Checking for updates…", "venter");
  const s = await invoke("status");
  klar = s.status !== "feil";
  if (s.status === "feil") {
    settStatus("Knox Life could not be reached", "feil", linje2([]));
    visNyheter([]);
    visMelding(s.melding);
  } else {
    const spill = [`Game ${s.spillversjon}`, `Patch ${s.bygg}`];
    visNyheter(s.nyheter);
    if (s.status === "nodmodus") {
      settStatus("Emergency mode – Knox Life runs without its patch for now", "venter", linje2(spill));
    } else if (s.status === "venter_pa_spillet") {
      settStatus("Knox Life is getting ready for the new game version", "venter", linje2(spill));
      visMelding(OPPDATERT);
      klar = false;
    } else {
      settStatus("Ready", "klar", linje2(spill));
    }
  }
  oppdaterPlay();
}

async function spill() {
  if (iGang) return;
  iGang = true;
  skjulMelding();
  const play = $("play");
  play.disabled = true;
  play.classList.add("i-gang");
  play.textContent = "Getting ready…";
  try {
    const utfall = await invoke("spill");
    if (utfall.utfall === "spilt" && !utfall.lastet) {
      visMelding(IKKE_LASTET, { rapport: true });
    }
  } catch (feil) {
    visMelding(String(feil), { rapport: true });
  } finally {
    iGang = false;
    oppdaterPlay();
    hentStatus();
  }
}

function åpneDetaljer(åpen) {
  $("detaljer").hidden = !åpen;
  $("slør").hidden = !åpen;
  if (åpen) $("detaljer-lukk").focus();
  else $("detaljer-knapp").focus();
}

async function kommando(navn, ferdig) {
  try {
    const svar = await invoke(navn);
    if (ferdig) visMelding(ferdig(svar), { ok: true });
  } catch (feil) {
    visMelding(String(feil), { rapport: navn !== "kopier_rapport" });
  }
}

async function start() {
  const o = await invoke("oversikt");
  steamNavn = o.steam_navn;
  $("versjon").textContent = `Launcher ${o.launcher}`;
  $("detaljer-versjon").textContent = `Launcher ${o.launcher}`;
  $("detaljlinje").textContent = linje2([]);
  if (!o.spillet) {
    visMelding("Project Zomboid was not found. Install it in Steam, then restart the launcher.");
  }
  await hentStatus();
}

listen("steg", (h) => {
  const s = h.payload;
  const tekst = (STEGENE[s.steg] || (() => "…"))(s);
  const play = $("play");
  play.textContent = tekst;
  if (s.steg === "ikke_lastet") visMelding(IKKE_LASTET, { rapport: true });
});

$("play").addEventListener("click", spill);
$("minimer").addEventListener("click", () => vinduet.minimize());
$("lukk").addEventListener("click", () => {
  // Mens spillet går, må launcheren være åpen for å sette filene tilbake etterpå.
  if (iGang) {
    visMelding("Knox Life puts the game files back when the game closes. Close the game first.");
    return;
  }
  vinduet.close();
});
$("melding-lukk").addEventListener("click", skjulMelding);
$("melding-rapport").addEventListener("click", () => kommando("kopier_rapport", () => "The report is copied. Paste it to staff on Discord."));
$("detaljer-knapp").addEventListener("click", () => åpneDetaljer(true));
$("detaljer-lukk").addEventListener("click", () => åpneDetaljer(false));
$("slør").addEventListener("click", () => åpneDetaljer(false));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("detaljer").hidden) åpneDetaljer(false);
});
$("kopier-id").addEventListener("click", () => kommando("kopier_steam_id", () => "Your Steam ID is copied."));
$("sett-tilbake").addEventListener("click", () =>
  kommando("sett_tilbake", (satt) =>
    satt ? "The game files are back as Steam installed them." : "Nothing to restore – the game files are as Steam installed them.",
  ),
);
$("kopier-rapport").addEventListener("click", () => kommando("kopier_rapport", () => "The report is copied. Paste it to staff on Discord."));
$("loggmappe").addEventListener("click", () => kommando("apne_loggmappen"));

start();
