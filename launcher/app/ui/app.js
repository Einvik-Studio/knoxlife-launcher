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
const IKKE_FUNNET = "Project Zomboid was not found. Choose the folder it is installed in.";
const IKKE_LASTET =
  'The Knox Life patch did not load. Press "Copy report" and send it to staff on Discord.';

const KO_TEKST = (plass, av) =>
  `The server is full. You are number ${plass} of ${av} in line – Knox Life starts by itself when it is your turn. Keep this window open.`;
const DIN_TUR = "It's your turn! Knox Life is starting and joins the server by itself. Your place is held for 5 minutes.";
const KO_SLUTT = {
  koen_er_full: "The line is full right now. Try again in a few minutes.",
  nede: "The server is restarting. Try again in a few minutes.",
  ikke_pa_whitelisten: "This Steam account is not on the whitelist. Apply on the website, or ask staff on Discord.",
};
const KO_FEIL = "The line could not be reached. Press Play to try again.";

let iGang = false;
let klar = false;
let spilletFunnet = true;
let server = null; // serverens status (spillere, omstart, full) – null når den ikke kan hentes
let iKo = false;

function settStatus(tekst, prikk, detaljer) {
  $("statustekst").textContent = tekst;
  $("prikk").className = "prikk" + (prikk ? " " + prikk : "");
  if (detaljer !== undefined) $("detaljlinje").textContent = detaljer;
}

function visMelding(tekst, { rapport = false, ok = false, velg = false, forlat = false } = {}) {
  $("meldingstekst").textContent = tekst;
  $("melding-rapport").hidden = !rapport;
  $("melding-velg").hidden = !velg;
  $("melding-forlat").hidden = !forlat;
  $("melding-lukk").hidden = forlat; // køen lukkes med «Leave line»
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
  play.disabled = iGang || iKo || !klar;
  if (!iGang && !iKo) {
    play.textContent = "PLAY";
    play.classList.remove("i-gang");
  }
}

let steamNavn = null;
let spillDeler = [];
function linje2(deler) {
  spillDeler = deler;
  const alle = [];
  const omstart = omstartTekst();
  if (omstart) alle.push(omstart);
  alle.push(...deler);
  if (steamNavn) alle.push(`Steam: ${steamNavn}`);
  return alle.join(" · ");
}

// Neste omstart i spillerens egen tid, når den er innen et døgn.
function omstartTekst() {
  if (!server || !server.omstart) return "";
  const om = server.omstart - Date.now();
  if (om <= 0 || om > 24 * 3600 * 1000) return "";
  const kl = new Date(server.omstart).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return `Restart at ${kl}`;
}

// Serverens tall bak «Ready»: spillerne, full, omstart eller nede.
function serverTekst() {
  if (!server) return "";
  if (!server.oppe) return "the server is down right now";
  if (server.stengt) return "the server is restarting";
  if (server.fullt) return `server full (${server.spillere}/${server.maks})${server.ko ? ` · ${server.ko} in line` : ""}`;
  return `${server.spillere}/${server.maks} players`;
}

let klarTekst = null; // statuslinjen når alt er klart («Ready» eller nødmodus), før serverens tall
function visKlar() {
  if (klarTekst === null || iGang || iKo) return;
  const tall = serverTekst();
  settStatus(tall ? `${klarTekst} · ${tall}` : klarTekst, klarTekst === "Ready" ? "klar" : "venter", linje2(spillDeler));
}

async function hentServer() {
  server = await invoke("serverstatus").catch(() => null);
  visKlar();
}

async function hentStatus() {
  settStatus("Checking for updates…", "venter");
  const s = await invoke("status");
  klar = s.status !== "feil";
  if (s.status === "feil") {
    settStatus("Knox Life could not be reached", "feil", linje2([]));
    visNyheter([]);
    // Statuslinjen sier at nettet svikter; meldingen viser det spilleren kan rette her og nå først.
    if (spilletFunnet) visMelding(s.melding);
    else visMelding(IKKE_FUNNET, { velg: true });
  } else {
    const spill = [`Game ${s.spillversjon}`, `Patch ${s.bygg}`];
    visNyheter(s.nyheter);
    klarTekst = null;
    if (s.status === "nodmodus") {
      klarTekst = "Emergency mode – Knox Life runs without its patch for now";
      settStatus(klarTekst, "venter", linje2(spill));
      hentServer();
    } else if (s.status === "venter_pa_spillet") {
      settStatus("Knox Life is getting ready for the new game version", "venter", linje2(spill));
      visMelding(OPPDATERT);
      klar = false;
    } else if (!spilletFunnet) {
      settStatus("Project Zomboid was not found", "feil", linje2(spill));
      klar = false;
    } else {
      klarTekst = "Ready";
      settStatus("Ready", "klar", linje2(spill));
      hentServer();
    }
  }
  oppdaterPlay();
}

// Køen (launcher.md 3.8 punkt 6): er serveren full, venter spilleren her – ikke i spillet. Launcheren spør hvert 10.
// sekund, og når det er spillerens tur, starter «Play» av seg selv.
let koTeller = 0; // hver kø har sitt nummer, så «Leave line» stopper den som går
async function stillIKo() {
  iKo = true;
  const denne = ++koTeller;
  const play = $("play");
  play.disabled = true;
  play.classList.add("i-gang");
  play.textContent = "IN LINE";
  settStatus("Waiting in line", "venter");
  visMelding("The server is full. Joining the line…", { forlat: true });
  let feil = 0;
  while (iKo && denne === koTeller) {
    let svar = null;
    try {
      svar = await invoke("ko");
      feil = 0;
    } catch {
      feil++;
      if (feil >= 3) {
        avsluttKo(KO_FEIL);
        return;
      }
    }
    if (denne !== koTeller || !iKo) return;
    if (svar && svar.ko === "klar") {
      iKo = false;
      oppdaterPlay();
      visMelding(DIN_TUR, { ok: true });
      spill({ fraKo: true });
      return;
    }
    if (svar && svar.ko === "venter") {
      play.textContent = `#${svar.plass} IN LINE`;
      settStatus(`Waiting in line – number ${svar.plass} of ${svar.av}`, "venter");
      visMelding(KO_TEKST(svar.plass, svar.av), { forlat: true });
    } else if (svar && KO_SLUTT[svar.ko]) {
      avsluttKo(KO_SLUTT[svar.ko]);
      return;
    }
    await new Promise((ferdig) => setTimeout(ferdig, 10000));
  }
}

function avsluttKo(tekst) {
  iKo = false;
  koTeller++;
  oppdaterPlay();
  visKlar();
  if (tekst) visMelding(tekst);
  else skjulMelding();
}

async function spill({ fraKo = false } = {}) {
  if (iGang || iKo) return;
  // Full server: først køen. Kan statusen ikke hentes, starter «Play» som før – spillet sier selv fra om det er fullt.
  if (!fraKo) {
    server = await invoke("serverstatus").catch(() => null);
    if (server && server.vent) {
      stillIKo();
      return;
    }
  }
  iGang = true;
  if (!fraKo) skjulMelding();
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
    // Fant ikke launcheren spillet, kan spilleren peke på mappen selv.
    const o = await invoke("oversikt").catch(() => ({ spillet: true }));
    spilletFunnet = o.spillet;
    visMelding(String(feil), { rapport: o.spillet, velg: !o.spillet });
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
  spilletFunnet = o.spillet;
  if (!o.spillet) {
    visMelding(IKKE_FUNNET, { velg: true });
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

$("play").addEventListener("click", () => spill());
$("melding-forlat").addEventListener("click", async () => {
  avsluttKo(null);
  await invoke("forlat_ko").catch(() => {});
});
// Statusen hentes på nytt hvert 30. sekund mens vinduet er åpent.
setInterval(() => {
  if (klar && !iGang && !iKo) hentServer();
}, 30000);
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
$("melding-velg").addEventListener("click", async () => {
  try {
    if (await invoke("velg_mappe")) {
      spilletFunnet = true;
      visMelding("Project Zomboid was found. Press Play when you are ready.", { ok: true });
      hentStatus();
    }
  } catch (feil) {
    visMelding(String(feil), { velg: true });
  }
});
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

// «Performance mode»: valget huskes av launcheren og gjelder fra neste «Play».
function visYtelse(på) {
  $("ytelse").setAttribute("aria-checked", på ? "true" : "false");
}
$("ytelse").addEventListener("click", async () => {
  const på = $("ytelse").getAttribute("aria-checked") !== "true";
  $("ytelse").disabled = true;
  try {
    await invoke("velg_ytelsesmodus", { paa: på });
    visYtelse(på);
    visMelding(
      på
        ? (iGang ? "Performance mode is on from the next time you press Play." : "Performance mode is on. It is used when you press Play.")
        : (iGang ? "Performance mode is off from the next time you press Play." : "Performance mode is off."),
      { ok: true },
    );
  } catch (feil) {
    visMelding(String(feil), { rapport: true });
  } finally {
    $("ytelse").disabled = false;
  }
});
invoke("ytelsesmodus_valgt").then(visYtelse).catch(() => visYtelse(false));

start();
