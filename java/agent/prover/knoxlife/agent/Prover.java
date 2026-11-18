package knoxlife.agent;

import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.FileTime;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/** Prøvene for agenten – en liten kjører i ren Java, ingen rammeverk. Hver statisk metode som heter {@code prøve…} kjøres. */
public final class Prover {
    private static Path tom;

    public static void main(String[] args) throws Exception {
        int feil = 0;
        int kjørt = 0;
        for (Method m : Prover.class.getDeclaredMethods()) {
            if (!m.getName().startsWith("prøve") || !Modifier.isStatic(m.getModifiers())) {
                continue;
            }
            kjørt++;
            tom = Files.createTempDirectory("knoxlife-prove");
            try {
                m.invoke(null);
                System.out.println("ok    " + m.getName());
            } catch (java.lang.reflect.InvocationTargetException e) {
                feil++;
                System.out.println("FEIL  " + m.getName() + ": " + e.getCause());
            }
        }
        System.out.println(kjørt + " prøver, " + feil + " feil");
        System.exit(feil == 0 && kjørt > 0 ? 0 : 1);
    }

    static void lik(Object venter, Object fikk) {
        if (!Objects.equals(venter, fikk)) {
            throw new AssertionError("ventet " + venter + ", fikk " + fikk);
        }
    }

    static void sant(boolean b, String hva) {
        if (!b) {
            throw new AssertionError(hva);
        }
    }

    static Javavakt.Inntrykk inntrykk(List<String> valg, List<String> sti, Map<String, String> miljø) {
        return new Javavakt.Inntrykk(valg, sti, miljø, "jdk.internal.loader.ClassLoaders$AppClassLoader", Boolean.FALSE,
                Path.of("/spill/KnoxLife/patch/12/knoxlife-agent.jar"));
    }

    // ---------- juksesperren ----------

    static void prøveVanligSpillGirIngenFunn() {
        var i = inntrykk(
                List.of("-javaagent:/spill/KnoxLife/patch/12/knoxlife-agent.jar", "-Dknoxlife.bygg=12", "-Xmx3072m",
                        "-XX:-EnableDynamicAgentLoading", "-Djava.awt.headless=true"),
                List.of("projectzomboid.jar", "/spill/KnoxLife/patch/12/knoxlife-agent.jar"),
                Map.of("PATH", "/usr/bin"));
        lik(List.of(), Javavakt.funn(i));
    }

    static void prøveFremmedAgentOgDebuggerFinnes() {
        var i = inntrykk(
                List.of("-javaagent:/spill/KnoxLife/patch/12/knoxlife-agent.jar",
                        "-javaagent:C:\\Users\\Bjørn\\Desktop\\viewpoint.jar=opt",
                        "-agentlib:jdwp=transport=dt_socket", "-agentpath:/x/libjuks.so", "-Xbootclasspath/a:/x"),
                List.of("projectzomboid.jar"), Map.of());
        lik(List.of("javaagent viewpoint.jar", "debugger", "native agent libjuks.so", "boot class path"), Javavakt.funn(i));
    }

    static void prøveMiljøvariablerUtenStier() {
        var i = inntrykk(List.of(), List.of("projectzomboid.jar"),
                Map.of("JAVA_TOOL_OPTIONS", "-javaagent:C:\\Users\\Bjørn\\juks.jar -Xmx1g"));
        lik(List.of("JAVA_TOOL_OPTIONS -javaagent:juks.jar -Xmx1g"), Javavakt.funn(i));
    }

    static void prøveKlassestienMedEkstraJarOgTomPost() {
        var i = inntrykk(List.of(), List.of("projectzomboid.jar", "", "/a/b/projectzomboid.jar", "/mods/zombiebuddy.jar", "/x/*"),
                Map.of());
        lik(List.of("class path: working folder", "class path projectzomboid.jar", "class path zombiebuddy.jar",
                "class path wildcard x"), Javavakt.funn(i));
    }

    static void prøveMappeMedKlasserFinnes() throws Exception {
        Path mappe = tom.resolve("spillet");
        Files.createDirectories(mappe.resolve("media/lua"));
        Files.writeString(mappe.resolve("media/lua/a.lua"), "x");
        var uten = inntrykk(List.of(), List.of("projectzomboid.jar", mappe.toString()), Map.of());
        lik(List.of(), Javavakt.funn(uten));
        Files.createDirectories(mappe.resolve("zombie/network"));
        Files.write(mappe.resolve("zombie/network/GameClient.class"), new byte[] {1});
        lik(List.of("classes in folder spillet: zombie/network/GameClient.class"), Javavakt.funn(uten));
    }

    static void prøveDynamiskeAgenterOgFremmedKlasselaster() {
        var i = new Javavakt.Inntrykk(List.of(), List.of("projectzomboid.jar"), Map.of(), "juks.Laster", Boolean.TRUE, null);
        lik(List.of("dynamic agents allowed", "class loader juks.Laster"), Javavakt.funn(i));
    }

    static void prøveHøystÅtteFunnOgKorteLinjerUtenSkilletegn() {
        List<String> valg = new ArrayList<>();
        for (int n = 0; n < 20; n++) {
            valg.add("-javaagent:/x/agent" + n + "|" + "a".repeat(200) + ".jar");
        }
        List<String> f = Javavakt.funn(inntrykk(valg, List.of("projectzomboid.jar"), Map.of()));
        lik(Javavakt.FLEST_FUNN, f.size());
        for (String s : f) {
            sant(s.length() <= Javavakt.LENGSTE_FUNN && !s.contains("|"), s);
        }
    }

    static void prøveVårAgentGjenkjennesSomFil() throws Exception {
        Path jar = tom.resolve("knoxlife-agent.jar");
        Files.write(jar, new byte[] {1});
        sant(Javavakt.erVår(tom.resolve(".").resolve("knoxlife-agent.jar").toString(), jar), "samme fil");
        sant(!Javavakt.erVår(tom.resolve("annen.jar").toString(), jar), "en annen fil");
    }

    // ---------- billetten og stempelet ----------

    static void prøveBillettenLesesEnGangOgSlettes() throws Exception {
        Path fil = tom.resolve("reise.txt");
        Files.writeString(fil, "adresse=play.example.invalid\nport=16261\nbillett=abc123\n");
        Billett b = Billett.hent(fil, Instant.now());
        lik(new Billett("play.example.invalid", 16261, "abc123"), b);
        sant(!Files.exists(fil), "filen er slettet");
        lik(null, Billett.hent(fil, Instant.now()));
        sant(!b.toString().contains("abc123"), "billetten skrives aldri ut");
    }

    static void prøveGammelEllerØdelagtBillettBrukesIkke() throws Exception {
        Path fil = tom.resolve("reise.txt");
        Files.writeString(fil, "adresse=a\nport=1\nbillett=b\n");
        Files.setLastModifiedTime(fil, FileTime.from(Instant.now().minusSeconds(11 * 60)));
        lik(null, Billett.hent(fil, Instant.now()));
        sant(!Files.exists(fil), "en gammel billett slettes også");
        lik(null, Billett.tolk("adresse=a\nport=99999\nbillett=b"));
        lik(null, Billett.tolk("adresse=a\nport=1"));
    }

    static void prøveStempeletOkOgFeil() throws Exception {
        Path fil = tom.resolve("KnoxLife").resolve("stempel.txt");
        Stempel.ok(fil, 14, 1000);
        lik("bygg=14\ntid=1000\n", Files.readString(fil));
        Stempel.feil(fil, new IllegalStateException("noe\ngikk galt ø"), 2000);
        String tekst = Files.readString(fil);
        sant(!tekst.contains("bygg="), "et stempel med feil har ikke bygg");
        lik("feil=IllegalStateException: noe gikk galt ?\ntid=2000\n", tekst);
    }

    static void prøveStempeletFraHjemmemappen() {
        Path hjem = tom.resolve("bjorn");
        lik(hjem.resolve("Zomboid/KnoxLife/stempel.txt"), Agent.sti("~/Zomboid/KnoxLife/stempel.txt", hjem.toString()));
        lik(Path.of("/a/b.txt"), Agent.sti("/a/b.txt", hjem.toString()));
    }

    static void prøveRapportlinjen() {
        KnoxLife.start(12);
        KnoxLife.settFunn(List.of("javaagent x.jar", "debugger"));
        lik("v1|12|ok|javaagent x.jar|debugger", KnoxLife.rapport());
        KnoxLife.settFunn(List.of());
        lik("v1|12|ok", KnoxLife.rapport());
        lik("v1|12|ok|debug mode", KnoxLife.rapport(true));
        KnoxLife.settFunn(List.of("f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8"));
        lik("v1|12|ok|debug mode|f1|f2|f3|f4|f5|f6|f7", KnoxLife.rapport(true));
        lik("v1|12|ok|f1|f2|f3|f4|f5|f6|f7|f8", KnoxLife.rapport(false));
        KnoxLife.settFunn(List.of());
    }

    // ---------- det Lua ser ----------

    static void prøveDeleneOgByggetForLua() {
        KnoxLife.start(12);
        lik(false, KnoxLifeJava.har("linje"));
        KnoxLife.del("linje");
        lik(true, KnoxLifeJava.har("linje"));
        lik(false, KnoxLifeJava.har(null));
        lik(12.0, KnoxLifeJava.bygg());
    }

    static void prøveHendelseskøenHolderRekkefølgenOgGrensen() {
        while (KnoxLifeJava.nesteHendelse() != null) {
            // tøm køen fra en annen prøve
        }
        KnoxLife.hendelse("{\"hendelse\":\"java_ok\"}");
        KnoxLife.hendelse("{\"hendelse\":\"java_debug\"}");
        lik("{\"hendelse\":\"java_ok\"}", KnoxLifeJava.nesteHendelse());
        lik("{\"hendelse\":\"java_debug\"}", KnoxLifeJava.nesteHendelse());
        lik(null, KnoxLifeJava.nesteHendelse());
        for (int i = 0; i < KnoxLife.FLEST_HENDELSER + 5; i++) {
            KnoxLife.hendelse("linje " + i);
        }
        lik("linje 5", KnoxLifeJava.nesteHendelse());
        int igjen = 1;
        while (KnoxLifeJava.nesteHendelse() != null) {
            igjen++;
        }
        lik(KnoxLife.FLEST_HENDELSER, igjen);
    }

    // ---------- J6: ny tilkobling etter omstart ----------

    static Tilbake.Innlogging inn() {
        return new Tilbake.Innlogging("ola", "hemmelig-hash", "10.0.0.5", "", "16261", "", "Knox Life", false, 1);
    }

    static final Tilbake.Meny MENYEN = new Tilbake.Meny(true, false, false, false);
    static final Tilbake.Meny LASTER = new Tilbake.Meny(false, false, false, false);
    static final Tilbake.Meny KOBLER = new Tilbake.Meny(false, true, true, false);
    static final Tilbake.Meny FEILET = new Tilbake.Meny(false, false, true, false);
    static final Tilbake.Meny INNE = new Tilbake.Meny(false, false, false, true);
    static final Tilbake.Meny AVBRUTT = new Tilbake.Meny(true, false, false, false);

    static void prøveTilbakeEtterOmstartKoblerTilHvert15Sekund() {
        var t = new Tilbake();
        t.påKnoxLife();
        t.omstart(true);
        sant(t.brudd(0, null, inn()), "Knox Life skal ta over etter en omstart");
        lik(Tilbake.OMSTART, t.skjermtekst());
        sant(!t.tilMenyen(2_999), "skjermen vises i 3 sekunder");
        sant(t.tilMenyen(3_000), "så går spillet til hovedmenyen");
        sant(!t.tilMenyen(3_001), "bare én gang");

        lik(Tilbake.Handling.INGENTING, t.iMenyen(3_050, LASTER).handling());
        var s = t.iMenyen(3_100, MENYEN);
        lik(Tilbake.Handling.KOBLE_TIL, s.handling());
        lik(1, t.forsøk());
        sant(s.tekst().startsWith(Tilbake.OMSTART), s.tekst());
        lik(Tilbake.Handling.INGENTING, t.iMenyen(3_200, KOBLER).handling()); // spillets egne beskjeder står

        // Forsøket feilet (serveren er ikke oppe ennå): vent 15 sekunder.
        s = t.iMenyen(10_000, FEILET);
        lik(Tilbake.Fase.VENTER, t.fase());
        sant(s.tekst().endsWith("Next try in 15 s."), s.tekst());
        sant(t.iMenyen(17_000, FEILET).tekst().endsWith("Next try in 8 s."), "nedtellingen");
        lik(Tilbake.Handling.KOBLE_TIL, t.iMenyen(25_000, FEILET).handling());
        lik(2, t.forsøk());

        // Andre forsøk går gjennom: skjermen lukker seg og spillet er koblet til.
        t.iMenyen(26_000, KOBLER);
        lik(Tilbake.Handling.INGENTING, t.iMenyen(27_000, INNE).handling());
        lik(Tilbake.Fase.FERDIG, t.fase());
        lik(null, t.innlogging());
        sant(!t.iGang(), "ferdig");
    }

    static void prøveTilbakeNårSpillerenGårTilMenyenSelv() {
        var t = new Tilbake();
        t.påKnoxLife();
        t.brudd(0, null, inn());
        // Spilleren trykket «Exit to main menu» før de 3 sekundene var gått.
        lik(Tilbake.Handling.KOBLE_TIL, t.iMenyen(1_000, MENYEN).handling());
        sant(!t.tilMenyen(5_000), "skjermen er alt borte");
    }

    static void prøveTilbakeGirOppEtter15Minutter() {
        var t = new Tilbake();
        t.påKnoxLife();
        sant(t.brudd(0, "", inn()), "et brudd uten grunn");
        lik(Tilbake.BRUDD, t.skjermtekst());
        t.tilMenyen(3_000);
        t.iMenyen(3_000, MENYEN);
        var s = t.iMenyen(Tilbake.FRIST_MS, FEILET);
        lik(Tilbake.GITT_OPP, s.tekst());
        lik(Tilbake.Fase.GITT_OPP, t.fase());
        lik(null, t.innlogging());
        lik(Tilbake.Handling.VIS_TEKST, t.iMenyen(Tilbake.FRIST_MS + 20_000, FEILET).handling());
        lik(Tilbake.Handling.INGENTING, t.iMenyen(Tilbake.FRIST_MS + 30_000, AVBRUTT).handling());

        // Menyen kom aldri fram (spilleren satt i et annet vindu): ingenting å vise, bare slutt.
        var u = new Tilbake();
        u.påKnoxLife();
        u.brudd(0, null, inn());
        u.tilMenyen(3_000);
        lik(Tilbake.Handling.INGENTING, u.iMenyen(Tilbake.FRIST_MS, LASTER).handling());
        lik(Tilbake.Fase.GITT_OPP, u.fase());
        lik(null, u.innlogging());
    }

    static void prøveTilbakeStopperNårSpillerenAvbryter() {
        var t = new Tilbake();
        t.påKnoxLife();
        t.brudd(0, null, inn());
        t.tilMenyen(3_000);
        t.iMenyen(3_000, MENYEN);
        t.iMenyen(5_000, FEILET); // feilet, venter
        lik(Tilbake.Handling.INGENTING, t.iMenyen(6_000, AVBRUTT).handling());
        lik(Tilbake.Fase.AVBRUTT, t.fase());
        lik(null, t.innlogging());
        lik(Tilbake.Handling.INGENTING, t.iMenyen(30_000, MENYEN).handling());

        // «Abort» mens spillet kobler til.
        var u = new Tilbake();
        u.påKnoxLife();
        u.brudd(0, null, inn());
        u.tilMenyen(3_000);
        u.iMenyen(3_000, MENYEN);
        u.iMenyen(3_100, KOBLER);
        u.iMenyen(3_200, AVBRUTT);
        lik(Tilbake.Fase.AVBRUTT, u.fase());
    }

    static void prøveTilbakeBareForKnoxLifeOgUtenUtkastelse() {
        var t = new Tilbake();
        sant(!t.brudd(0, null, inn()), "en annen server enn Knox Life");
        t.påKnoxLife();
        sant(!t.brudd(0, "You have been kicked from this server by an admin.", inn()), "kastet ut");
        lik(Tilbake.Fase.AV, t.fase());
        sant(!t.brudd(0, null, inn()), "påKnoxLife gjelder bare én innlogging");
        t.påKnoxLife();
        t.omstart(true);
        sant(t.brudd(0, "Server stopped.", inn()), "etter et varsel om omstart gjelder spillets egen grunn også");
        t.påKnoxLife(); // inne igjen
        lik(Tilbake.Fase.FERDIG, t.fase());
        lik(null, t.innlogging());
        lik(Tilbake.BRUDD, t.skjermtekst()); // varselet gjaldt bare forrige innlogging

        // Omstarten ble avlyst: den som kastes ut etterpå, får spillets vanlige skjerm.
        t.omstart(true);
        t.omstart(false);
        sant(!t.brudd(0, "You have been kicked from this server by an admin.", inn()), "avlyst omstart");
        // Et varsel før modet har sagt at dette er Knox Life, gjelder ikke.
        t.omstart(true);
        t.påKnoxLife();
        lik(Tilbake.BRUDD, t.skjermtekst());
    }

    // ---------- J5: overgangen til testserveren ----------

    static final Servere.Server TEST = new Servere.Server("test", "10.0.0.6", 16271);

    static void prøveServereFraLauncheren() {
        var s = Servere.tolk("hoved=play.example.org:16261\r\ntest=10.0.0.6:16271\n\n# kommentar\nTEST=x:1\nfeil=:16261\n"
                + "port=a.b:0\nport2=a.b:99999\nrom=a b:16261\nhoved=annen.example.org:1\n");
        lik(2, s.alle().size());
        lik(new Servere.Server("hoved", "play.example.org", 16261), s.finn("hoved"));
        lik(new Servere.Server("test", "10.0.0.6", 16271), s.finn("test"));
        lik(null, s.finn("TEST"));
        lik(null, s.finn(null));
        lik(null, Servere.tolk(""));
        lik(null, Servere.tolk("ingenting her"));
        lik("the Knox Life test server", s.finn("test").tittel());
        lik("Knox Life", s.finn("hoved").tittel());
        StringBuilder mange = new StringBuilder();
        for (char c = 'a'; c < 'a' + 12; c++) {
            mange.append(c).append("=h:1\n");
        }
        lik(Servere.FLEST, Servere.tolk(mange.toString()).alle().size());
    }

    static void prøveServereLesesFraFilenOgBlirStående() throws Exception {
        Path fil = tom.resolve("servere.txt");
        lik(null, Servere.les(fil));
        Files.writeString(fil, "test=10.0.0.6:16271\n");
        lik(TEST, Servere.les(fil).finn("test"));
        sant(Files.exists(fil), "listen står til launcheren skriver den på nytt");
        Files.writeString(fil, "x".repeat(5000));
        lik(null, Servere.les(fil));
    }

    static void prøveReiseGårTilTestserverenMedDenSammeInnloggingen() {
        var t = new Tilbake();
        sant(!t.reise(0, inn(), TEST), "bare på Knox Life");
        t.påKnoxLife();
        sant(!t.reise(0, null, TEST), "uten innlogging");
        sant(!t.reise(0, inn(), null), "uten mål");
        sant(t.reise(1_000, inn(), TEST), "på Knox Life med mål");
        lik(Tilbake.Fase.MENYEN, t.fase());
        lik(TEST, t.reisemål());
        var i = t.innlogging();
        lik("ola", i.bruker());
        lik("hemmelig-hash", i.passord());
        lik("10.0.0.6", i.ip());
        lik("16271", i.port());
        lik("", i.serverpassord());
        lik(false, i.steamRelay());
        lik(1, i.autentisering());
        lik("Going to the Knox Life test server – connecting automatically.", t.skjermtekst());
        // Hovedmenyen: spillet kobler til testserveren med en gang den er klar.
        lik(Tilbake.Handling.INGENTING, t.iMenyen(1_500, LASTER).handling());
        var s = t.iMenyen(2_000, MENYEN);
        lik(Tilbake.Handling.KOBLE_TIL, s.handling());
        sant(s.tekst().startsWith("Going to the Knox Life test server"), s.tekst());
        lik(Tilbake.Handling.INGENTING, t.iMenyen(2_100, KOBLER).handling());
        lik(Tilbake.Handling.INGENTING, t.iMenyen(9_000, INNE).handling());
        lik(Tilbake.Fase.FERDIG, t.fase());
        // Inne på testserveren: modet sier at dette er Knox Life, og et brudd der gir veien tilbake til den.
        t.påKnoxLife();
        lik(null, t.reisemål());
        sant(t.brudd(20_000, null, new Tilbake.Innlogging("ola", "h", "10.0.0.6", "", "16271", "", "", false, 1)), "brudd på testserveren");
        lik(Tilbake.BRUDD, t.skjermtekst());
        lik("10.0.0.6", t.innlogging().ip());
    }

    static void prøveReiseMedSkjermenVedBruddet() {
        // Viser spillet skjermen for bruddet på vei ut, står målet: teksten sier hvor det går, og så hovedmenyen.
        var t = new Tilbake();
        t.påKnoxLife();
        t.reise(0, inn(), TEST);
        sant(t.brudd(500, "Server stopped.", inn()), "brudd på vei ut");
        lik(Tilbake.Fase.SKJERMEN, t.fase());
        lik(TEST, t.reisemål());
        lik("Going to the Knox Life test server – connecting automatically.", t.skjermtekst());
        sant(!t.tilMenyen(2_000), "skjermen står litt");
        sant(t.tilMenyen(Tilbake.PÅ_SKJERMEN_MS), "så hovedmenyen");
        lik(Tilbake.Handling.KOBLE_TIL, t.iMenyen(4_000, MENYEN).handling());
        lik("10.0.0.6", t.innlogging().ip());
    }

    static void prøveReiseGirOppNårBillettenErUte() {
        var t = new Tilbake();
        t.påKnoxLife();
        t.reise(0, inn(), TEST);
        t.iMenyen(1_000, MENYEN);
        t.iMenyen(1_100, FEILET);
        lik(Tilbake.Fase.VENTER, t.fase());
        lik(Tilbake.Handling.KOBLE_TIL, t.iMenyen(Tilbake.REISE_FRIST_MS, FEILET).handling()); // det siste forsøket
        var s = t.iMenyen(Tilbake.REISE_FRIST_MS + 100, FEILET);
        lik(Tilbake.Fase.GITT_OPP, t.fase());
        lik("The Knox Life test server did not let you in. Join Knox Life again from the server list.", s.tekst());
        lik(Tilbake.Handling.VIS_TEKST, t.iMenyen(Tilbake.REISE_FRIST_MS + 1_000, FEILET).handling());
        // Et brudd etterpå er et vanlig brudd (spilleren kom inn på hovedserveren selv og mistet forbindelsen).
        t.påKnoxLife();
        sant(t.brudd(Tilbake.REISE_FRIST_MS + 60_000, null, inn()), "vanlig brudd");
        lik(Tilbake.BRUDD, t.skjermtekst());
        lik("10.0.0.5", t.innlogging().ip());
    }

    static void prøveReiseSomBleLiggendeGjelderIkkeEtBruddSenere() {
        // Spillet gikk aldri til menyen (noe feilet): et brudd etter fristen er et vanlig brudd.
        var t = new Tilbake();
        t.påKnoxLife();
        t.reise(0, inn(), TEST);
        sant(t.brudd(Tilbake.REISE_FRIST_MS, null, inn()), "brudd lenge etter");
        lik(null, t.reisemål());
        lik(Tilbake.BRUDD, t.skjermtekst());
        lik("10.0.0.5", t.innlogging().ip());
    }

    static void prøveReiseFraLuaUtenListeEllerSpilletSierNei() {
        // Uten listen fra launcheren (eller uten Java-modene for spillet) går ingen reise.
        lik(false, KnoxLifeJava.reise("test"));
        lik(false, KnoxLifeJava.reise(null));
        lik(null, Tilbake.fraSpillet()); // knoxlife.tilbake.Spillet er ikke bygget inn i prøvene
    }

    static void prøveTilbakeViserAldriPassordet() {
        sant(!inn().toString().contains("hemmelig"), inn().toString());
    }

    // ---------- J7 profileren og J9 hendelsene som hoppes over ----------

    static void prøveFunksjonenFraStien() {
        lik(new Profil.Funksjon("KnoxLife", "server/KnoxLife/BildeServer.lua", 186), Profil.Funksjon.av(
                "C:\\Users\\spiller\\Zomboid\\mods\\KnoxLife\\42.0\\media\\lua\\server\\KnoxLife\\BildeServer.lua", 186));
        lik(new Profil.Funksjon("Tow", "client/TowCar.lua", 12), Profil.Funksjon.av(
                "/steam/workshop/content/108600/3000000001/mods/Tow/42/media/lua/client/TowCar.lua", 12));
        lik(new Profil.Funksjon("game", "shared/Items/ItemPicker.lua", 3),
                Profil.Funksjon.av("@media/lua/shared/Items/ItemPicker.lua", 3));
        lik(new Profil.Funksjon("Tow", "client/TowCar.lua", 0), Profil.Funksjon.av("mods/Tow/media/lua/client/TowCar.lua", -4));
        lik(new Profil.Funksjon("game", "Fil.lua", 1), Profil.Funksjon.av("Fil.lua", 1));
        lik(Profil.Funksjon.UKJENT, Profil.Funksjon.av(null, 1));
        lik("KnoxLife server/X.lua:7", new Profil.Funksjon("KnoxLife", "server/X.lua", 7).tekst());
    }

    /** En profil der funksjonene heter det nøkkelen sier («mod fil:linje»). */
    static Profil profil() {
        var p = new Profil();
        p.navngi(n -> {
            String[] d = String.valueOf(n).split(" ");
            int kolon = d[1].lastIndexOf(':');
            return new Profil.Funksjon(d[0], d[1].substring(0, kolon), Integer.parseInt(d[1].substring(kolon + 1)));
        });
        return p;
    }

    static void kall(Profil p, Object hendelse, String navn, Object funksjon, long ns) {
        Object t = p.før(hendelse, navn, funksjon);
        if (t != null) {
            p.etter(t, ns);
        }
    }

    static void prøveProfilenTellerOgGirDeDyresteFørst() {
        var p = profil();
        Object tikk = new Object();
        Object kommando = new Object();
        p.på(true, 1_000);
        for (int i = 0; i < 10; i++) {
            kall(p, tikk, "OnTick", "ModA a.lua:1", 1_000_000);
            kall(p, tikk, "OnTick", "ModB b.lua:5", 100_000);
        }
        kall(p, kommando, "OnClientCommand", "ModA a.lua:1", 30_000_000); // den samme funksjonen på en annen hendelse
        kall(p, kommando, "OnClientCommand", "ModC c.lua:9", 2_000_000);
        Profil.Minutt m = p.minutt(61_000);
        lik(1_000L, m.fraMs());
        lik(61_000L, m.tilMs());
        lik(22L, m.kall());
        lik(10_000_000L + 1_000_000L + 30_000_000L + 2_000_000L, m.luaNs());
        lik(4, m.dyrest().size());
        lik("OnClientCommand", m.dyrest().get(0).hendelse());
        lik(new Profil.Funksjon("ModA", "a.lua", 1), m.dyrest().get(0).funksjon());
        lik("OnTick", m.dyrest().get(1).hendelse());
        lik(10L, m.dyrest().get(1).kall());
        lik(1_000_000L, m.dyrest().get(1).maksNs());
        lik(new Profil.Funksjon("ModC", "c.lua", 9), m.dyrest().get(2).funksjon());
        // Tellingen begynner på nytt.
        Profil.Minutt neste = p.minutt(121_000);
        lik(61_000L, neste.fraMs());
        lik(0, neste.dyrest().size());
        lik(0L, neste.kall());
    }

    static void prøveProfilenGirHøystTjueOgFårEnTekstMedRareTegn() {
        var p = profil();
        Object h = new Object();
        p.på(true, 0);
        for (int i = 1; i <= 30; i++) {
            kall(p, h, "OnTick", "Mod f" + i + ".lua:" + i, i * 1_000L);
        }
        kall(p, h, "On\"Rar\nHendelse\\", "Mod rar.lua:2", 5);
        Profil.Minutt m = p.minutt(60_000);
        lik(Profil.FLEST, m.dyrest().size());
        lik(30, m.dyrest().get(0).funksjon().linje());
        String json = m.json();
        sant(json.startsWith("{\"fra\":0,\"til\":60000,\"kall\":31,"), json);
        sant(json.contains("\"hendelse\":\"OnTick\",\"mod\":\"Mod\",\"fil\":\"f30.lua\",\"linje\":30,\"kall\":1,\"ms\":0.030,"), json);
        sant(!json.contains("\n"), "ingen linjeskift");
    }

    static void prøveProfilenAvTellerIngenting() {
        var p = profil();
        Object h = new Object();
        Object t = p.før(h, "OnTick", "Mod a.lua:1");
        sant(t != null, "kallet går");
        p.etter(t, 5_000);
        p.hendelse(Profil.TIKK, 0);
        p.hendelse(Profil.TIKK, 900_000_000L);
        Profil.Minutt m = p.minutt(60_000);
        lik(0L, m.kall());
        lik(0, m.tikk());
        lik(0, m.dyrest().size());
    }

    static void prøveTikkeneMåles() {
        var p = profil();
        p.på(true, 0);
        long ns = 0;
        p.hendelse(Profil.TIKK, ns);
        for (int i = 0; i < 9; i++) {
            ns += 100_000_000L;
            p.hendelse(Profil.TIKK, ns);
        }
        ns += 450_000_000L;
        p.hendelse(Profil.TIKK, ns); // et tregt tikk
        p.hendelse("OnTick", ns + 1); // andre hendelser teller ikke
        Profil.Minutt m = p.minutt(60_000);
        lik(10, m.tikk());
        lik(450_000_000L, m.tikkMaksNs());
        lik(1, m.tregeTikk());
        // Det første tikket etter et nytt minutt måler ikke fra et gammelt tikk.
        p.hendelse(Profil.TIKK, ns + 10_000_000_000L);
        lik(0, p.minutt(120_000).tikk());
    }

    static void prøveHoppLinjenLesesBareMedGrunn() {
        var h = Profil.Hopp.tolk("OnTick ZombieMod client/Horde.lua:42 # Går gjennom alle zombiene hvert tikk; vi har ingen.");
        lik(new Profil.Hopp("OnTick", "ZombieMod", "client/Horde.lua", 42,
                "Går gjennom alle zombiene hvert tikk; vi har ingen."), h);
        lik("OnTick ZombieMod client/Horde.lua:42 # Går gjennom alle zombiene hvert tikk; vi har ingen.", h.somLinje());
        lik(null, Profil.Hopp.tolk("OnTick ZombieMod client/Horde.lua:42"));
        lik(null, Profil.Hopp.tolk("OnTick ZombieMod client/Horde.lua:42 #   "));
        lik(null, Profil.Hopp.tolk("OnTick client/Horde.lua:42 # mangler moden"));
        lik(null, Profil.Hopp.tolk("OnTick ZombieMod client/Horde.lua # mangler linjen"));
        lik(null, Profil.Hopp.tolk("OnTick ZombieMod client/Horde.lua:x # ikke et tall"));
        lik(null, Profil.Hopp.tolk(null));
    }

    static void prøveHendelserIListenHoppesOverOgTelles() {
        var p = profil();
        Object h = new Object();
        // Listen gjelder også når profileren står av.
        var tatt = p.hopp(java.util.Arrays.asList(Profil.Hopp.tolk("OnTick ZombieMod z.lua:4 # zombier"), null));
        lik(1, tatt.size());
        lik(null, p.før(h, "OnTick", "ZombieMod z.lua:4"));
        lik(null, p.før(h, "OnTick", "zombiemod z.lua:4")); // moden med små bokstaver er den samme
        sant(p.før(h, "OnTick", "ZombieMod z.lua:5") != null, "en annen linje kalles");
        sant(p.før(new Object(), "OnPlayerUpdate", "ZombieMod z.lua:4") != null, "en annen hendelse kalles");
        p.på(true, 0);
        lik(null, p.før(h, "OnTick", "ZombieMod z.lua:4"));
        lik(1L, p.minutt(60_000).hoppet());
        // En ny liste uten linjen: funksjonen kalles igjen.
        p.hopp(java.util.List.of());
        sant(p.før(h, "OnTick", "ZombieMod z.lua:4") != null, "kalles igjen");
        // Høyst FLEST_HOPP linjer.
        var mange = new java.util.ArrayList<Profil.Hopp>();
        for (int i = 0; i < Profil.FLEST_HOPP + 5; i++) {
            mange.add(new Profil.Hopp("OnTick", "M", "f.lua", i, "grunn"));
        }
        lik(Profil.FLEST_HOPP, p.hopp(mange).size());
    }

    static void prøveMenyenTømmerListenOgSlårAvProfileren() {
        Profil p = new Profil();
        p.på(true, 0);
        p.hopp(java.util.List.of(new Profil.Hopp("OnTick", "M", "f.lua", 1, "grunn")));
        p.iMenyen();
        sant(!p.på(), "profileren er av");
        lik(0, p.hopp().size());
        // Funksjonen på listen kalles igjen (ute av Knox Life).
        Object f = new Object();
        p.navngi(x -> new Profil.Funksjon("M", "f.lua", 1));
        sant(p.før("OnTick", "OnTick", f) != null, "kalles");
        // Serveren: ingenting endres.
        Profil s = new Profil();
        s.på(true, 0);
        s.hopp(java.util.List.of(new Profil.Hopp("OnTick", "M", "f.lua", 1, "grunn")));
        s.lås();
        s.iMenyen();
        sant(s.på(), "står på");
        lik(1, s.hopp().size());
    }

    static void prøveProfilenFraLuaHosSpillerenOgLåstPåServeren() {
        // Hos spilleren: modet slår den på og henter minuttet.
        lik(null, KnoxLifeJava.profilMinutt());
        lik(true, KnoxLifeJava.profil(true));
        sant(KnoxLifeJava.profilMinutt().startsWith("{\"fra\":"), "minuttet");
        lik(1.0, KnoxLifeJava.hopp("OnTick M f.lua:1 # grunn\nikke en linje\n"));
        lik("OnTick M f.lua:1 # grunn\n", KnoxLifeJava.hoppListe());
        lik(true, KnoxLifeJava.profil(false));
        lik(0.0, KnoxLifeJava.hopp(null));
        // Serveren låser: Lua kan ikke endre noe.
        Profil.SPILLET.lås();
        Profil.SPILLET.på(true, 0);
        lik(false, KnoxLifeJava.profil(false));
        sant(Profil.SPILLET.på(), "står på");
        lik(null, KnoxLifeJava.profilMinutt());
        lik(-1.0, KnoxLifeJava.hopp("OnTick M f.lua:1 # grunn"));
    }
}
