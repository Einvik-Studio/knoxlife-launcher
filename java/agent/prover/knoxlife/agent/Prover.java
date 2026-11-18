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

    static void prøveTilbakeViserAldriPassordet() {
        sant(!inn().toString().contains("hemmelig"), inn().toString());
    }
}
