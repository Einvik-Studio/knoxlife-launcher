package knoxlife.agent;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicInteger;

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
    private static final Set<String> DELER = ConcurrentHashMap.newKeySet();
    /** Linjene Lua skal føre i loggen ({@code server/KnoxLife/JavaServer.lua} i modet henter dem hvert tikk). */
    private static final ConcurrentLinkedQueue<String> HENDELSER = new ConcurrentLinkedQueue<>();
    private static final AtomicInteger I_KØ = new AtomicInteger();
    /** Flest linjer i køen. Henter ikke Lua dem (modet er ikke lastet), går de eldste tapt i stedet for minnet. */
    public static final int FLEST_HENDELSER = 10_000;

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

    /** En Java-del er lastet og virker («juksesperre», «linje» …). Modet spør med {@code KnoxLife.Java.har}. */
    public static void del(String navn) {
        DELER.add(navn);
    }

    public static boolean har(String navn) {
        return navn != null && DELER.contains(navn);
    }

    /** En linje til loggen i modet: {@code {"hendelse":"…","data":{…}}}. */
    public static void hendelse(String jsonLinje) {
        HENDELSER.add(jsonLinje);
        if (I_KØ.incrementAndGet() > FLEST_HENDELSER && HENDELSER.poll() != null) {
            I_KØ.decrementAndGet();
        }
    }

    /** Den eldste linjen i køen, eller {@code null}. */
    public static String nesteHendelse() {
        String linje = HENDELSER.poll();
        if (linje != null) {
            I_KØ.decrementAndGet();
        }
        return linje;
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
