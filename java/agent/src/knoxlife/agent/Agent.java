package knoxlife.agent;

import java.lang.instrument.Instrumentation;
import java.net.URISyntaxException;
import java.nio.file.Path;
import java.time.Instant;

/**
 * Knox Life-agenten. Java laster den før spillet ({@code -javaagent:} først i spillets valg, lagt inn av launcheren).
 *
 * <p>I denne rekkefølgen: juksesperren ser gjennom Java-en, billetten hentes, Mixin startes med Java-modene våre, og
 * stempelet skrives til slutt – så launcheren bare ser «lastet» når alt over gikk bra.
 */
public final class Agent {
    private static volatile boolean startet;

    private Agent() {}

    public static boolean startet() {
        return startet;
    }

    public static void premain(String argumenter, Instrumentation inst) {
        long bygg = Long.getLong("knoxlife.bygg", 0L);
        String stempelSti = System.getProperty("knoxlife.stempel");
        Path stempel = stempelSti == null || stempelSti.isBlank() ? null : sti(stempelSti, System.getProperty("user.home"));
        try {
            startet = true;
            KnoxLife.start(bygg);
            KnoxLife.settFunn(Javavakt.funn(Javavakt.Inntrykk.herfra(egenJar())));
            KnoxLife.del("juksesperre");
            if (stempel != null) {
                KnoxLife.settBillett(Billett.hent(stempel.resolveSibling("reise.txt"), Instant.now()));
            }
            knoxlife.agent.mixin.Start.start(inst);
            if (stempel != null) {
                Stempel.ok(stempel, bygg, System.currentTimeMillis());
            }
            System.out.println("[KnoxLife] agent loaded, build " + bygg);
        } catch (Throwable t) {
            KnoxLife.feil("agent", t);
            if (stempel != null) {
                Stempel.feil(stempel, t, System.currentTimeMillis());
            }
        }
    }

    /**
     * En sti fra launcheren. {@code ~/} foran betyr spillerens hjemmemappe: launcheren skriver stempelet slik, så et
     * brukernavn med «ø» aldri står i spillets fil (Windows leser den i et gammelt tegnsett), og Java finner mappen selv.
     */
    static Path sti(String verdi, String hjem) {
        if (verdi.startsWith("~/") && hjem != null) {
            return Path.of(hjem).resolve(verdi.substring(2));
        }
        return Path.of(verdi);
    }

    /** Agentens egen jar-fil. */
    static Path egenJar() {
        try {
            return Path.of(Agent.class.getProtectionDomain().getCodeSource().getLocation().toURI());
        } catch (URISyntaxException | RuntimeException e) {
            return null;
        }
    }
}
