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
        Path stempel = stempelSti == null || stempelSti.isBlank() ? null : Path.of(stempelSti);
        try {
            startet = true;
            KnoxLife.start(bygg);
            KnoxLife.settFunn(Javavakt.funn(Javavakt.Inntrykk.herfra(egenJar())));
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

    /** Agentens egen jar-fil. */
    static Path egenJar() {
        try {
            return Path.of(Agent.class.getProtectionDomain().getCodeSource().getLocation().toURI());
        } catch (URISyntaxException | RuntimeException e) {
            return null;
        }
    }
}
