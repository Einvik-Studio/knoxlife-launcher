package knoxlife.agent;

import java.nio.file.Path;
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
    private static volatile Servere servere;
    private static volatile Servere.Server rettInn;
    private static volatile Path mappe;
    private static final List<String> FEIL = Collections.synchronizedList(new ArrayList<>());
    private static final Set<String> DELER = ConcurrentHashMap.newKeySet();
    private static final Set<String> ENDRET = ConcurrentHashMap.newKeySet();
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

    static void settMappe(Path m) {
        mappe = m;
    }

    /** Mappen launcheren og agenten deler ({@code Zomboid/KnoxLife}), eller {@code null} uten launcheren. */
    public static Path mappe() {
        return mappe;
    }

    static void settRettInn(Servere.Server s) {
        rettInn = s;
    }

    static void settServere(Servere s) {
        servere = s;
    }

    /** Knox Lifes server med navnet fra launcherens liste ({@link Servere}), eller {@code null}. */
    public static Servere.Server server(String navn) {
        Servere s = servere;
        return s == null ? null : s.finn(navn);
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

    /** En av spillets klasser fikk Java-modene sine da den ble lastet. Porten sjekker at hver klasse som skal, fikk det. */
    public static void endret(String klasse) {
        ENDRET.add(klasse);
    }

    public static Set<String> endret() {
        return Set.copyOf(ENDRET);
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

    /** Serveren «Play» skal rett inn på ({@link RettInn}) – hentes én gang i hovedmenyen, så tas den bort. */
    public static synchronized Servere.Server taRettInn() {
        Servere.Server s = rettInn;
        rettInn = null;
        return s;
    }

    /** Funnet som sier at spillet går i debug-modus. Serveren har sin egen regel for det (launcher.md 3.3 punkt 2). */
    public static final String DEBUG = "debug mode";

    public static String rapport() {
        return rapport(false);
    }

    /** Rapporten slik den sendes i innloggingen. Debug-modus står først, så den alltid er med innenfor grensen. */
    public static String rapport(boolean debug) {
        StringBuilder s = new StringBuilder("v1|").append(bygg).append('|').append(FEIL.isEmpty() ? "ok" : "feil");
        int n = 0;
        if (debug) {
            s.append('|').append(DEBUG);
            n++;
        }
        for (String f : funn) {
            if (n++ >= Javavakt.FLEST_FUNN) {
                break;
            }
            s.append('|').append(f);
        }
        return s.toString();
    }
}
