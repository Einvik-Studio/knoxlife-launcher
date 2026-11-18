package knoxlife.agent;

/**
 * J6, ny tilkobling etter omstart eller brudd (launcher.md 3.6 punkt 7): når forbindelsen til Knox Life faller, går
 * spillet selv til hovedmenyen og kobler til den samme serveren igjen, hvert 15. sekund i opptil 15 minutter.
 *
 * <p>Her er bare avgjørelsene, uten spillet, så de kan prøves for seg. Java-modene for skjermen ved bruddet og
 * hovedmenyen ({@code knoxlife.tilbake.Spillet}) spør her og gjør det som står i svaret.
 *
 * <p>Bare en server modet har sagt er Knox Life ({@link #påKnoxLife()}) gir en ny tilkobling, og bare etter et brudd
 * uten grunn (nettet falt, serveren stoppet) eller en omstart serveren har sagt fra om ({@link #omstart(boolean)}).
 * Den som blir kastet ut eller utestengt, får en grunn med i bruddet og spillets vanlige skjerm.
 */
public final class Tilbake {
    /** Så lenge skjermen ved bruddet vises før spillet går til hovedmenyen selv. */
    public static final long PÅ_SKJERMEN_MS = 3_000;
    /** Tiden mellom to forsøk. */
    public static final long MELLOM_FORSØK_MS = 15_000;
    /** Så lenge det prøves fra bruddet. */
    public static final long FRIST_MS = 15 * 60_000;

    /** Det som vises når serveren har sagt fra om en omstart. */
    public static final String OMSTART = "Server restarting – you will be back in automatically.";
    /** Det som vises når forbindelsen falt uten varsel. */
    public static final String BRUDD = "Connection to Knox Life lost – you will be back in automatically.";
    /** Det som vises når 15 minutter har gått. */
    public static final String GITT_OPP = "Knox Life is not back yet. Join again from the server list in a few minutes.";

    /**
     * Innloggingen spilleren alt brukte, slik spillet har den i minnet ({@code GameClient}; passordet er alt gjort om
     * av spillet). Den skrives aldri til disk, og {@link #toString()} har aldri passordet.
     */
    public record Innlogging(String bruker, String passord, String ip, String lokalIp, String port, String serverpassord,
            String servernavn, boolean steamRelay, int autentisering) {
        @Override
        public String toString() {
            return bruker + "@" + ip + ":" + port;
        }
    }

    /**
     * Hovedmenyen slik spillet viser den nå. {@code klar}: menyen er fremme og kan brukes; {@code kobler}: spillet
     * prøver å koble til; {@code synlig}: skjermen for tilkoblingen vises; {@code tilkoblet}: spillet har en
     * forbindelse til serveren.
     */
    public record Meny(boolean klar, boolean kobler, boolean synlig, boolean tilkoblet) {
    }

    public enum Fase { AV, SKJERMEN, MENYEN, KOBLER, VENTER, FERDIG, GITT_OPP, AVBRUTT }

    public enum Handling {
        /** Ingenting å gjøre. */
        INGENTING,
        /** Koble til med {@link #innlogging()}, og vis teksten. */
        KOBLE_TIL,
        /** Vis teksten i stedet for spillets egen. */
        VIS_TEKST
    }

    public record Steg(Handling handling, String tekst) {
        static final Steg INGENTING = new Steg(Handling.INGENTING, null);
    }

    /** Den ene for spillet; prøvene lager sine egne. */
    public static final Tilbake SPILLET = new Tilbake();

    private boolean påKnoxLife;
    private boolean omstart;
    private Fase fase = Fase.AV;
    private Innlogging innlogging;
    private long bruddTid;
    private long neste;
    private int forsøk;

    /** Modet er i spillet på Knox Life. Kalles ved hver innlogging, også etter en ny tilkobling. */
    public synchronized void påKnoxLife() {
        påKnoxLife = true;
        omstart = false;
        if (fase != Fase.AV) {
            ferdig(Fase.FERDIG);
        }
    }

    /** Serveren har sagt fra at den starter på nytt ({@code true}), eller at omstarten er avlyst ({@code false}). */
    public synchronized void omstart(boolean kommer) {
        omstart = påKnoxLife && kommer;
    }

    /**
     * Forbindelsen falt, og spillet viser skjermen for bruddet. {@code true} når Knox Life tar over: skjermen viser
     * {@link #skjermtekst()} og går selv til hovedmenyen.
     */
    public synchronized boolean brudd(long nå, String grunn, Innlogging inn) {
        // Spillet gir en grunn bare når serveren kaster spilleren ut. Har serveren sagt fra om omstarten, gjelder
        // også den grunnen spillet får når serveren stopper.
        boolean vår = omstart || grunn == null || grunn.isBlank();
        boolean tar = påKnoxLife && vår && inn != null;
        påKnoxLife = false;
        if (!tar) {
            omstart = false;
            fase = Fase.AV;
            innlogging = null;
            return false;
        }
        innlogging = inn;
        bruddTid = nå;
        forsøk = 0;
        fase = Fase.SKJERMEN;
        return true;
    }

    public synchronized String skjermtekst() {
        return omstart ? OMSTART : BRUDD;
    }

    /** {@code true} én gang, når skjermen har vist seg lenge nok og spillet skal gå til hovedmenyen. */
    public synchronized boolean tilMenyen(long nå) {
        if (fase == Fase.SKJERMEN && nå - bruddTid >= PÅ_SKJERMEN_MS) {
            fase = Fase.MENYEN;
            return true;
        }
        return false;
    }

    /** Hvert bilde i hovedmenyen. */
    public synchronized Steg iMenyen(long nå, Meny m) {
        switch (fase) {
            // SKJERMEN: spilleren gikk til menyen selv før de 3 sekundene var gått.
            case SKJERMEN, MENYEN -> {
                if (nå - bruddTid >= FRIST_MS) {
                    ferdig(Fase.GITT_OPP);
                    return Steg.INGENTING;
                }
                return m.klar() ? kobleTil() : Steg.INGENTING;
            }
            case KOBLER -> {
                if (m.kobler()) {
                    // Spillets egne beskjeder (nedlasting av mods, køen) står til forsøket er over.
                    return Steg.INGENTING;
                }
                if (!m.synlig()) {
                    // Skjermen lukket seg: spillet gikk videre inn, eller spilleren trykket «Abort».
                    ferdig(m.tilkoblet() ? Fase.FERDIG : Fase.AVBRUTT);
                    return Steg.INGENTING;
                }
                if (nå - bruddTid >= FRIST_MS) {
                    ferdig(Fase.GITT_OPP);
                    return new Steg(Handling.VIS_TEKST, GITT_OPP);
                }
                fase = Fase.VENTER;
                neste = nå + MELLOM_FORSØK_MS;
                return venter(nå);
            }
            case VENTER -> {
                if (!m.synlig()) {
                    ferdig(Fase.AVBRUTT);
                    return Steg.INGENTING;
                }
                return nå >= neste ? kobleTil() : venter(nå);
            }
            case GITT_OPP -> {
                return m.synlig() ? new Steg(Handling.VIS_TEKST, GITT_OPP) : Steg.INGENTING;
            }
            default -> {
                return Steg.INGENTING;
            }
        }
    }

    /** Noe i spillet feilet: ingen ny tilkobling denne gangen. */
    public synchronized void stopp() {
        if (fase != Fase.AV) {
            ferdig(Fase.AVBRUTT);
        }
    }

    private void ferdig(Fase slutt) {
        fase = slutt;
        innlogging = null;
    }

    private Steg kobleTil() {
        fase = Fase.KOBLER;
        forsøk++;
        return new Steg(Handling.KOBLE_TIL, skjermtekst() + " Connecting (attempt " + forsøk + ")…");
    }

    private Steg venter(long nå) {
        long sekunder = Math.max(1, (neste - nå + 999) / 1000);
        return new Steg(Handling.VIS_TEKST, skjermtekst() + " Next try in " + sekunder + " s.");
    }

    public synchronized Fase fase() {
        return fase;
    }

    /** Er det noe å gjøre i hovedmenyen? Spørres hvert bilde, så det er billig. */
    public synchronized boolean iGang() {
        return switch (fase) {
            case SKJERMEN, MENYEN, KOBLER, VENTER, GITT_OPP -> true;
            default -> false;
        };
    }

    public synchronized Innlogging innlogging() {
        return innlogging;
    }

    public synchronized int forsøk() {
        return forsøk;
    }
}
