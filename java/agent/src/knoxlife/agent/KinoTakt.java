package knoxlife.agent;

/**
 * Hvor i filmen kinoen er (J11). Lua sier hvor filmen skal være etter serverens klokke ({@link #vis}); her går den jevnt
 * videre på egen klokke, og hopper bare når avviket er over {@link #AVVIK} – ellers ville bildet rykke ved hver melding.
 */
final class KinoTakt {
    /** Sekunder avvik før filmen hopper (kino.md del 5). */
    static final double AVVIK = 0.5;

    private String film = "";
    private double grunnSek;
    private long grunnMs;
    private boolean gaar;
    private long hopp;

    /**
     * Filmen {@code id} skal være {@code sek} inn nå ({@code naaMs}, egen klokke). Svarer {@code true} når filmen byttet
     * eller hoppet – da må lyden flyttes også.
     */
    synchronized boolean vis(String id, double sek, boolean gaarNaa, long naaMs) {
        String ny = id == null ? "" : id;
        double s = Math.max(0, sek);
        boolean hoppet;
        if (!ny.equals(film)) {
            hoppet = true;
        } else {
            hoppet = Math.abs(posisjon(naaMs) - s) > AVVIK || gaar != gaarNaa;
        }
        if (hoppet || ny.isEmpty()) {
            film = ny;
            grunnSek = s;
            grunnMs = naaMs;
            gaar = gaarNaa;
            hopp++;
        }
        return hoppet;
    }

    synchronized double posisjon(long naaMs) {
        if (!gaar) {
            return grunnSek;
        }
        return grunnSek + Math.max(0, naaMs - grunnMs) / 1000.0;
    }

    synchronized String film() {
        return film;
    }

    synchronized boolean gaar() {
        return gaar;
    }

    /** Teller som øker ved hvert hopp – lyden ser på den. */
    synchronized long hopp() {
        return hopp;
    }
}
