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
    }
}
