package knoxlife.agent;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Det agenten vet mens spillet går, ett sted Java-modene kan spørre.
 *
 * <p>Rapporten til juksesperren er én linje: {@code v1|<bygg>|<ok eller feil>|<funn>|<funn>…}. Funnene har aldri
 * {@code |} i seg, og høyst {@link Javavakt#FLEST_FUNN} står med.
 */
public final class KnoxLife {
    private static volatile long bygg;
    private static volatile List<String> funn = List.of();
    private static volatile Billett billett;
    private static final List<String> FEIL = Collections.synchronizedList(new ArrayList<>());

    private KnoxLife() {}

    static void start(long byggnummer) {
        bygg = byggnummer;
    }

    static void settFunn(List<String> nye) {
        funn = List.copyOf(nye);
    }

    static void settBillett(Billett b) {
        billett = b;
    }

    /** En Java-mod som ikke kunne legges inn. Skrives i spillets konsoll og sier fra i rapporten. */
    public static void feil(String hvor, Throwable t) {
        String linje = hvor + ": " + t;
        if (FEIL.size() < 50) {
            FEIL.add(linje);
        }
        System.err.println("[KnoxLife] " + linje);
        for (Throwable årsak = t.getCause(); årsak != null && årsak != årsak.getCause(); årsak = årsak.getCause()) {
            System.err.println("[KnoxLife]   because " + årsak);
        }
        if (Boolean.getBoolean("knoxlife.feilsok")) {
            t.printStackTrace();
        }
    }

    public static long bygg() {
        return bygg;
    }

    public static List<String> funn() {
        return funn;
    }

    public static List<String> feil() {
        synchronized (FEIL) {
            return List.copyOf(FEIL);
        }
    }

    /** Billetten fra launcheren – hentes én gang, så tas den bort. */
    public static synchronized Billett taBillett() {
        Billett b = billett;
        billett = null;
        return b;
    }

    public static String rapport() {
        StringBuilder s = new StringBuilder("v1|").append(bygg).append('|').append(FEIL.isEmpty() ? "ok" : "feil");
        for (String f : funn) {
            s.append('|').append(f);
        }
        return s.toString();
    }
}
