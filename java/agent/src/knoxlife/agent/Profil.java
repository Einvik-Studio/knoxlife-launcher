package knoxlife.agent;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.function.Function;

/**
 * J7 profileren og J9 hendelsene som hoppes over (launcher.md 3.8 punkt 1 og 4).
 *
 * <p>Spillet kaller hver Lua-funksjon som lytter på en hendelse ({@code Events.OnTick.Add(...)}), én etter én.
 * {@code ProfilMixin} spør her før hvert kall ({@link #før}) og sier fra etterpå ({@link #etter}): to klokkeavlesninger
 * og noen plusstykker per kall, og ingenting nytt i minnet når en funksjon er sett én gang. Hvert minutt hentes de
 * dyreste ut ({@link #minutt}). På serveren står profileren på hele tiden; hos en spiller bare når staff ber om det.
 *
 * <p>Her er bare tellingen, uten spillet, så den kan prøves for seg: hendelsen og funksjonen er bare nøkler (spillets
 * hendelse og Kahluas {@code Prototype}), og {@link #navngi} sier hvilken fil og linje en funksjon er.
 */
public final class Profil {
    /** Så mange av de dyreste per minutt. */
    public static final int FLEST = 20;
    /** Et tikk som tar lengre enn dette, er tregt (serveren tikker hvert 100. ms, en spiller hver frame). */
    public static final long TREGT_TIKK_NS = 200_000_000L;
    /** Høyst så mange linjer i listen over hendelser som hoppes over. */
    public static final int FLEST_HOPP = 100;
    /** Hendelsen som går hvert tikk også når serveren står tom (launcher.md: OnTick går ikke på en tom server). */
    public static final String TIKK = "OnTickEvenPaused";

    /** Den ene for spillet; prøvene lager sine egne. */
    public static final Profil SPILLET = new Profil();

    /** Svaret fra {@link #før} når ingenting skal telles (profileren står av og listen er tom). */
    private static final Object INGEN = new Object();

    /**
     * En Lua-funksjon: moden («game» for spillets egne filer), filen fra {@code media/lua/} og linjen funksjonen
     * starter på.
     */
    public record Funksjon(String mod, String fil, int linje) {
        public static final Funksjon UKJENT = new Funksjon("?", "?", 0);

        /**
         * Fra stien Kahlua har for funksjonen og den første linjen i den. Moden er mappen etter den siste
         * {@code /mods/} (også i Workshop-mappen); uten den er det spillets egen fil.
         */
        public static Funksjon av(String sti, int linje) {
            if (sti == null || sti.isBlank()) {
                return UKJENT;
            }
            String s = sti.replace('\\', '/');
            if (s.startsWith("@")) {
                s = s.substring(1);
            }
            if (!s.startsWith("/")) {
                s = "/" + s; // «mods/X/…» uten mappen foran
            }
            String mod = "game";
            int mods = s.lastIndexOf("/mods/");
            if (mods >= 0) {
                int fra = mods + "/mods/".length();
                int til = s.indexOf('/', fra);
                if (til > fra) {
                    mod = s.substring(fra, til);
                }
            }
            String fil = s;
            int lua = s.lastIndexOf("media/lua/");
            if (lua >= 0) {
                fil = s.substring(lua + "media/lua/".length());
            } else {
                int skråstrek = s.lastIndexOf('/');
                fil = skråstrek >= 0 ? s.substring(skråstrek + 1) : s;
            }
            return new Funksjon(kort(mod, 64), kort(fil, 160), Math.max(0, linje));
        }

        /** Slik staff ser funksjonen og skriver den i listen over hendelser som hoppes over. */
        public String tekst() {
            return mod + " " + fil + ":" + linje;
        }
    }

    /** Én funksjon på én hendelse i ett minutt. Tidene i nanosekunder. */
    public record Rad(String hendelse, Funksjon funksjon, long kall, long ns, long maksNs) {
    }

    /**
     * Et minutt: de dyreste funksjonene, alle kallene og all tiden i Lua-hendelsene, tikkene (antall, det lengste og
     * hvor mange som var trege) og hvor mange kall som ble hoppet over.
     */
    public record Minutt(long fraMs, long tilMs, List<Rad> dyrest, long kall, long luaNs, int tikk, long tikkMaksNs,
            int tregeTikk, long hoppet) {
        /** Som én linje JSON (tidene i millisekunder med tre desimaler). */
        public String json() {
            StringBuilder b = new StringBuilder(256 + dyrest.size() * 128);
            b.append("{\"fra\":").append(fraMs).append(",\"til\":").append(tilMs)
                    .append(",\"kall\":").append(kall).append(",\"lua_ms\":").append(ms(luaNs))
                    .append(",\"tikk\":").append(tikk).append(",\"tikk_maks_ms\":").append(ms(tikkMaksNs))
                    .append(",\"trege_tikk\":").append(tregeTikk).append(",\"hoppet\":").append(hoppet)
                    .append(",\"dyrest\":[");
            for (int i = 0; i < dyrest.size(); i++) {
                Rad r = dyrest.get(i);
                if (i > 0) {
                    b.append(',');
                }
                b.append("{\"hendelse\":");
                tekst(b, r.hendelse());
                b.append(",\"mod\":");
                tekst(b, r.funksjon().mod());
                b.append(",\"fil\":");
                tekst(b, r.funksjon().fil());
                b.append(",\"linje\":").append(r.funksjon().linje()).append(",\"kall\":").append(r.kall())
                        .append(",\"ms\":").append(ms(r.ns())).append(",\"maks_ms\":").append(ms(r.maksNs()))
                        .append('}');
            }
            return b.append("]}").toString();
        }
    }

    /**
     * Én linje i listen over hendelser som hoppes over (J9): {@code <hendelse> <mod> <fil>:<linje> # <grunn>}. Uten
     * grunn tas linjen ikke med – hver linje skal si hvorfor.
     */
    public record Hopp(String hendelse, String mod, String fil, int linje, String grunn) {
        /** {@code null} når linjen ikke stemmer. */
        public static Hopp tolk(String linje) {
            if (linje == null) {
                return null;
            }
            int kommentar = linje.indexOf('#');
            if (kommentar < 0) {
                return null;
            }
            String grunn = linje.substring(kommentar + 1).strip();
            String[] deler = linje.substring(0, kommentar).strip().split("\\s+");
            if (grunn.isEmpty() || deler.length != 3) {
                return null;
            }
            int kolon = deler[2].lastIndexOf(':');
            if (kolon <= 0) {
                return null;
            }
            int nr;
            try {
                nr = Integer.parseInt(deler[2].substring(kolon + 1));
            } catch (NumberFormatException e) {
                return null;
            }
            if (nr < 0 || deler[0].length() > 80 || deler[1].length() > 64 || kolon > 160) {
                return null;
            }
            return new Hopp(deler[0], deler[1], deler[2].substring(0, kolon), nr, kort(grunn, 200));
        }

        /** Slik linjen står i listen. */
        public String somLinje() {
            return hendelse + " " + mod + " " + fil + ":" + linje + " # " + grunn;
        }

        boolean gjelder(String hendelse, Funksjon f) {
            return this.hendelse.equals(hendelse) && mod.equalsIgnoreCase(f.mod()) && fil.equals(f.fil())
                    && linje == f.linje();
        }
    }

    private static final class Teller {
        final String hendelse;
        final Object funksjon;
        long kall;
        long ns;
        long maks;
        /** Listen telleren sist ble sjekket mot, og svaret. */
        int hoppSjekket = -1;
        boolean hoppes;

        Teller(String hendelse, Object funksjon) {
            this.hendelse = hendelse;
            this.funksjon = funksjon;
        }
    }

    private final Map<Object, Map<Object, Teller>> tellere = new IdentityHashMap<>();
    private final Map<Object, Funksjon> navn = new IdentityHashMap<>();
    private volatile Function<Object, Funksjon> navngi;
    private boolean navnLastet;
    private volatile boolean på;
    private volatile boolean låst;
    private volatile boolean hopperOver;
    private List<Hopp> hopp = List.of();
    private int hoppNr;
    private long fraMs = -1;
    private long kall;
    private long luaNs;
    private long hoppet;
    private long sisteTikk = -1;
    private int tikk;
    private long tikkMaks;
    private int trege;

    /** Hvordan en funksjon (nøkkelen spillet gir) blir til fil og linje. Settes av Java-moden. */
    public void navngi(Function<Object, Funksjon> n) {
        navngi = n;
    }

    /** Slår tellingen på eller av. På igjen begynner et nytt minutt. */
    public synchronized void på(boolean p, long nåMs) {
        if (p && !på) {
            nullstill(nåMs);
        }
        på = p;
    }

    public boolean på() {
        return på;
    }

    /**
     * Serveren: profileren står på hele tiden, og listen kommer bare fra serverens {@code java.ini} – Lua kan ikke
     * endre noen av dem ({@link KnoxLifeJava}).
     */
    public void lås() {
        låst = true;
    }

    public boolean låst() {
        return låst;
    }

    /** Ny liste over hendelser som hoppes over (J9). Gir linjene som ble tatt med. */
    public synchronized List<Hopp> hopp(List<Hopp> nye) {
        List<Hopp> l = new ArrayList<>();
        for (Hopp h : nye) {
            if (h != null && l.size() < FLEST_HOPP) {
                l.add(h);
            }
        }
        hopp = List.copyOf(l);
        hoppNr++;
        hopperOver = !hopp.isEmpty();
        return hopp;
    }

    public synchronized List<Hopp> hopp() {
        return hopp;
    }

    /**
     * Før spillet kaller {@code funksjon} for {@code hendelse}: {@code null} når kallet skal hoppes over (J9), ellers
     * det som gis til {@link #etter}.
     */
    public Object før(Object hendelse, String hendelsesnavn, Object funksjon) {
        if (!på && !hopperOver) {
            return INGEN;
        }
        synchronized (this) {
            Map<Object, Teller> m = tellere.get(hendelse);
            if (m == null) {
                m = new IdentityHashMap<>();
                tellere.put(hendelse, m);
            }
            Teller t = m.get(funksjon);
            if (t == null) {
                t = new Teller(hendelsesnavn == null ? "?" : hendelsesnavn, funksjon);
                m.put(funksjon, t);
            }
            if (!hopp.isEmpty()) {
                if (t.hoppSjekket != hoppNr) {
                    t.hoppSjekket = hoppNr;
                    t.hoppes = false;
                    Funksjon f = funksjon(funksjon);
                    for (Hopp h : hopp) {
                        if (h.gjelder(t.hendelse, f)) {
                            t.hoppes = true;
                            break;
                        }
                    }
                }
                if (t.hoppes) {
                    hoppet++;
                    return null;
                }
            }
            return t;
        }
    }

    /** Etter kallet: tiden det tok. */
    public void etter(Object fraFør, long ns) {
        if (fraFør == INGEN || !(fraFør instanceof Teller t) || !på) {
            return;
        }
        synchronized (this) {
            t.kall++;
            t.ns += ns;
            if (ns > t.maks) {
                t.maks = ns;
            }
            kall++;
            luaNs += ns;
        }
    }

    /** En hendelse begynner (før funksjonene kalles): tikket måles fra {@link #TIKK} til den neste. */
    public void hendelse(String hendelsesnavn, long nåNs) {
        if (!på || !TIKK.equals(hendelsesnavn)) {
            return;
        }
        synchronized (this) {
            if (sisteTikk >= 0) {
                long tid = nåNs - sisteTikk;
                tikk++;
                if (tid > tikkMaks) {
                    tikkMaks = tid;
                }
                if (tid > TREGT_TIKK_NS) {
                    trege++;
                }
            }
            sisteTikk = nåNs;
        }
    }

    /**
     * Minuttet som har gått siden sist: de {@link #FLEST} dyreste, og alt annet. Tellingen begynner på nytt. Når
     * profileren står av, er listen tom.
     */
    public synchronized Minutt minutt(long nåMs) {
        List<Teller> brukt = new ArrayList<>();
        for (Map<Object, Teller> m : tellere.values()) {
            for (Teller t : m.values()) {
                if (t.kall > 0) {
                    brukt.add(t);
                }
            }
        }
        brukt.sort(Comparator.comparingLong((Teller t) -> t.ns).reversed());
        List<Rad> dyrest = new ArrayList<>();
        for (int i = 0; i < brukt.size() && i < FLEST; i++) {
            Teller t = brukt.get(i);
            dyrest.add(new Rad(t.hendelse, funksjon(t.funksjon), t.kall, t.ns, t.maks));
        }
        Minutt m = new Minutt(fraMs < 0 ? nåMs : fraMs, nåMs, List.copyOf(dyrest), kall, luaNs, tikk, tikkMaks,
                trege, hoppet);
        nullstill(nåMs);
        return m;
    }

    private void nullstill(long nåMs) {
        for (Map<Object, Teller> m : tellere.values()) {
            for (Teller t : m.values()) {
                t.kall = 0;
                t.ns = 0;
                t.maks = 0;
            }
        }
        fraMs = nåMs;
        kall = 0;
        luaNs = 0;
        hoppet = 0;
        tikk = 0;
        tikkMaks = 0;
        trege = 0;
        sisteTikk = -1;
    }

    private Funksjon funksjon(Object nøkkel) {
        Funksjon f = navn.get(nøkkel);
        if (f == null) {
            if (navngi == null && !navnLastet) {
                navnLastet = true;
                try {
                    // Bygget mot spillet (Java-modene), og melder seg selv når den lastes.
                    Class.forName("knoxlife.profil.Navn", true, Profil.class.getClassLoader());
                } catch (ClassNotFoundException | LinkageError e) {
                    // uten spillet (prøvene): funksjonene står som ukjente
                }
            }
            Function<Object, Funksjon> n = navngi;
            try {
                f = n == null ? null : n.apply(nøkkel);
            } catch (RuntimeException e) {
                f = null;
            }
            if (f == null) {
                f = Funksjon.UKJENT;
            }
            navn.put(nøkkel, f);
        }
        return f;
    }

    static String kort(String s, int høyst) {
        String t = s.replaceAll("[\\p{Cntrl}\"\\\\]", "");
        return t.length() > høyst ? t.substring(0, høyst) : t;
    }

    private static String ms(long ns) {
        return String.format(Locale.ROOT, "%.3f", ns / 1_000_000.0);
    }

    private static void tekst(StringBuilder b, String s) {
        b.append('"').append(kort(s, 200)).append('"');
    }
}
