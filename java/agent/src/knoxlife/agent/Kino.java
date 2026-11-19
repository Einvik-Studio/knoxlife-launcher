package knoxlife.agent;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.LongSupplier;
import java.util.regex.Pattern;

/**
 * Kinoen hos spilleren (J11, kino.md 3.9): hvilke filmer som er lovlige, hvilken som går og hvor, lerretet, lyset,
 * fokuset og lyden. Lua styrer alt gjennom {@link KnoxKino}; tegningen ({@code knoxlife.kino.Lerret}) og lyden
 * ({@link Lyd}) ligger i Java-modene mot spillet. Bildene leses i en egen tråd, og bare mens lerretet er tegnet det
 * siste {@link #SYNLIG_MS} – langt unna gjør kinoen ingenting.
 */
public final class Kino {
    public static final Kino SPILLET = new Kino(System::currentTimeMillis);

    /** Hvor lenge etter siste tegning lerretet regnes som synlig. */
    static final long SYNLIG_MS = 2000;
    /** Tre buffere på rad: den spillet laster opp, blir ikke skrevet over før to nye bilder er lest. */
    static final int BUFFERE = 3;
    private static final Pattern ID = Pattern.compile("[a-z0-9_-]{1,40}");
    private static final Pattern SHA = Pattern.compile("[0-9a-f]{64}");

    /** Lyden i spillet (FMOD), lagt inn av Java-modene. Kalles bare fra spillets tråd. */
    public interface Lyd {
        void spill(Path ogg, long ms);

        void pause(boolean pause);

        void volum(float volum);

        void stopp();
    }

    record Film(String sha, long bytes) {}

    /** Et ferdig bilde til lerretet: bufferen (RGBA, {@link KinoBilde#BREDDE} × {@link KinoBilde#HOYDE}) og nummeret. */
    public record Ramme(ByteBuffer data, long nr) {}

    private final LongSupplier klokke;
    private final Map<String, Film> filmer = new ConcurrentHashMap<>();
    private final Map<String, Boolean> sjekket = new ConcurrentHashMap<>();
    private final Map<String, Boolean> lydKlar = new ConcurrentHashMap<>();
    private final KinoTakt takt = new KinoTakt();
    private final ExecutorService bakgrunn = Executors.newSingleThreadExecutor(r -> {
        Thread t = new Thread(r, "KnoxLife kino");
        t.setDaemon(true);
        t.setPriority(Thread.MIN_PRIORITY);
        return t;
    });
    private final ByteBuffer[] buffere = new ByteBuffer[BUFFERE];
    private volatile Path mappe;
    private volatile double[] lerret;
    private volatile String tekst = "";
    private volatile double lys = 1;
    private volatile double fokus = 1;
    private volatile long sistTegnet = Long.MIN_VALUE / 2;
    private volatile Ramme ramme;
    private long nr;
    private Thread leser;
    private Lyd lyd;
    private long lydHopp = -1;
    private String lydFilm = "";
    private boolean lydPause;

    Kino(LongSupplier klokke) {
        this.klokke = klokke;
    }

    // ---------- fra Lua (KnoxKino) ----------

    void film(String id, String sha, double bytes) {
        if (id == null || sha == null || !ID.matcher(id).matches() || !SHA.matcher(sha).matches() || bytes <= 0) {
            return;
        }
        Film ny = new Film(sha, (long) bytes);
        if (!ny.equals(filmer.put(id, ny))) {
            sjekket.remove(id);
        }
    }

    /** Filen finnes og har riktig sjekksum. Første gang sjekkes den i bakgrunnen, og svaret er {@code false} til da. */
    boolean har(String id) {
        if (id == null || !filmer.containsKey(id)) {
            return false;
        }
        Boolean svar = sjekket.get(id);
        if (svar != null) {
            return svar;
        }
        if (sjekket.putIfAbsent(id, Boolean.FALSE) == null) {
            bakgrunn.submit(() -> sjekket.put(id, sjekk(id)));
        }
        return false;
    }

    boolean sjekk(String id) {
        Film f = filmer.get(id);
        Path p = fil(id);
        try {
            if (f == null || p == null || !Files.isRegularFile(p) || Files.size(p) != f.bytes()) {
                return false;
            }
            return f.sha().equals(KinoFil.sha256(p));
        } catch (IOException e) {
            return false;
        }
    }

    void lerret(double x1, double y1, double x2, double y2, double z, double etasjer) {
        if (etasjer <= 0 || etasjer > 8) {
            return;
        }
        lerret = new double[] {x1, y1, x2, y2, z, etasjer};
    }

    void vis(String id, double sek, boolean gaar) {
        String f = id == null || filmer.containsKey(id) ? id : null;
        takt.vis(f, sek, gaar, klokke.getAsLong());
    }

    void tekst(String linje) {
        tekst = linje == null ? "" : linje.length() > 80 ? linje.substring(0, 80) : linje;
    }

    void bilde(double l, double f) {
        lys = Math.max(0, Math.min(1, l));
        fokus = Math.max(0, Math.min(1, f));
    }

    /**
     * Volumet nå (Lua regner det ut hvert sekund). Her startes, flyttes og stoppes lyden – alltid i spillets tråd.
     */
    void lyd(double volum) {
        Lyd l = lyd;
        if (l == null) {
            return;
        }
        String film = takt.film();
        boolean gaar = takt.gaar();
        if (film.isEmpty() || !har(film) || volum <= 0 && !lydFilm.isEmpty()) {
            if (!lydFilm.isEmpty()) {
                l.stopp();
                lydFilm = "";
            }
            return;
        }
        if (volum <= 0) {
            return;
        }
        Path ogg = lydFil(film);
        if (ogg == null) {
            return; // skrives ut i bakgrunnen; neste sekund
        }
        long hopp = takt.hopp();
        if (!film.equals(lydFilm) || hopp != lydHopp) {
            l.spill(ogg, (long) (takt.posisjon(klokke.getAsLong()) * 1000));
            lydFilm = film;
            lydHopp = hopp;
            lydPause = false;
        }
        if (lydPause == gaar) {
            l.pause(!gaar);
            lydPause = !gaar;
        }
        l.volum((float) Math.min(1, volum));
    }

    /** Lydsporet som egen fil, skrevet én gang i bakgrunnen; {@code null} til den er klar. */
    Path lydFil(String id) {
        Path knf = fil(id);
        if (knf == null) {
            return null;
        }
        Path ogg = knf.resolveSibling(id + ".ogg");
        if (Boolean.TRUE.equals(lydKlar.get(id))) {
            return ogg;
        }
        if (lydKlar.putIfAbsent(id, Boolean.FALSE) == null) {
            bakgrunn.submit(() -> {
                try (KinoFil k = KinoFil.åpne(knf)) {
                    if (!Files.isRegularFile(ogg) || Files.size(ogg) != k.hode().lydBytes()) {
                        k.lydTil(ogg);
                    }
                    lydKlar.put(id, Boolean.TRUE);
                } catch (IOException e) {
                    lydKlar.remove(id);
                }
            });
        }
        return null;
    }

    // ---------- til tegningen ----------

    /** Lerretet ({@code x1, y1, x2, y2, z, etasjer}), eller {@code null} når Lua ikke har sagt hvor det er. */
    public double[] lerret() {
        return lerret;
    }

    /** Er det noe å vise: en film som er lastet ned, eller en tekst? */
    public boolean aktiv() {
        if (lerret == null) {
            return false;
        }
        String film = takt.film();
        return !tekst.isEmpty() || !film.isEmpty() && Boolean.TRUE.equals(sjekket.get(film));
    }

    public double lys() {
        return lys;
    }

    /** Lerretet blir tegnet nå: lesingen av bildene holdes i gang. Gir det nyeste bildet, eller {@code null}. */
    public Ramme tegnes() {
        sistTegnet = klokke.getAsLong();
        startLeser();
        return ramme;
    }

    boolean synlig() {
        return klokke.getAsLong() - sistTegnet < SYNLIG_MS;
    }

    public synchronized void settLyd(Lyd l) {
        lyd = l;
    }

    void settMappe(Path m) {
        mappe = m;
    }

    Path fil(String id) {
        Path m = mappe != null ? mappe : KnoxLife.mappe();
        return m == null ? null : m.resolve("kino").resolve(id + ".knf");
    }

    // ---------- lesingen av bildene ----------

    private synchronized void startLeser() {
        if (leser != null) {
            return;
        }
        leser = new Thread(this::les, "KnoxLife kino bilder");
        leser.setDaemon(true);
        leser.setPriority(Thread.NORM_PRIORITY - 1);
        leser.start();
    }

    private void les() {
        KinoFil åpen = null;
        String åpenId = "";
        String sistTekst = null;
        int sistBilde = -1;
        int sistRadius = -1;
        while (true) {
            try {
                if (!synlig()) {
                    if (åpen != null) {
                        åpen.close();
                        åpen = null;
                        åpenId = "";
                    }
                    sistTekst = null;
                    sistBilde = -1;
                    Thread.sleep(250);
                    continue;
                }
                String t = tekst;
                int r = KinoBilde.radius(fokus);
                if (!t.isEmpty()) {
                    if (!t.equals(sistTekst)) {
                        ByteBuffer b = neste();
                        KinoBilde.tekst(t, b);
                        publiser(b);
                        sistTekst = t;
                        sistBilde = -1;
                    }
                    Thread.sleep(100);
                    continue;
                }
                sistTekst = null;
                String film = takt.film();
                if (film.isEmpty() || !Boolean.TRUE.equals(sjekket.get(film))) {
                    Thread.sleep(100);
                    continue;
                }
                if (!film.equals(åpenId)) {
                    if (åpen != null) {
                        åpen.close();
                    }
                    åpen = KinoFil.åpne(fil(film));
                    åpenId = film;
                    sistBilde = -1;
                }
                double pos = takt.posisjon(klokke.getAsLong());
                int i = (int) Math.min(åpen.hode().bilder() - 1, Math.floor(pos * åpen.hode().fps()));
                if (i != sistBilde || r != sistRadius) {
                    ByteBuffer b = neste();
                    KinoBilde.jpeg(åpen.bilde(i), r, b);
                    publiser(b);
                    sistBilde = i;
                    sistRadius = r;
                }
                Thread.sleep(Math.max(5, (long) (500 / åpen.hode().fps())));
            } catch (InterruptedException e) {
                return;
            } catch (IOException | RuntimeException e) {
                KnoxLife.feil("kino", e);
                try {
                    if (åpen != null) {
                        åpen.close();
                    }
                    Thread.sleep(2000);
                } catch (IOException | InterruptedException ignorert) {
                    // prøver igjen
                }
                åpen = null;
                åpenId = "";
            }
        }
    }

    private ByteBuffer neste() {
        int i = (int) ((nr + 1) % BUFFERE);
        if (buffere[i] == null) {
            buffere[i] = KinoBilde.nyBuffer();
        }
        return buffere[i];
    }

    private void publiser(ByteBuffer b) {
        nr++;
        ramme = new Ramme(b, nr);
    }

    // ---------- prøvene ----------

    double posisjon() {
        return takt.posisjon(klokke.getAsLong());
    }

    String filmNaa() {
        return takt.film();
    }

    String tekstNaa() {
        return tekst;
    }
}
