package knoxlife.agent;

/**
 * Det Lua ser av kinoen (J11, kino.md 3.9), som den globale {@code KnoxKino}. Modet ({@code client/KnoxLife/KinoKlient.lua})
 * sier hvilke filmer som er lovlige, hvor lerretet står, hvilken film som går og hvor langt den er kommet, lyset og
 * fokuset, og volumet hvert sekund. Ingenting her bestemmer over noe: en annen mod som kaller det, endrer bare det
 * spilleren selv ser og hører. Tallene er {@code double}, slik Lua har dem.
 */
public final class KnoxKino {
    private KnoxKino() {}

    public static String versjon() {
        return "1";
    }

    /** En lovlig film fra modets liste: id, SHA-256 og størrelsen i bytes. Bare slike filer vises. */
    public static void film(String id, String sha256, double bytes) {
        Kino.SPILLET.film(id, sha256, bytes);
    }

    /** Filen {@code Zomboid/KnoxLife/kino/<id>.knf} finnes og har riktig sjekksum (sjekkes én gang i bakgrunnen). */
    public static boolean har(String id) {
        return Kino.SPILLET.har(id);
    }

    /** Lerretets nedre kant i verden fra (x1, y1) til (x2, y2) på etasje z, {@code etasjer} høyt. */
    public static void lerret(double x1, double y1, double x2, double y2, double z, double etasjer) {
        Kino.SPILLET.lerret(x1, y1, x2, y2, z, etasjer);
    }

    /** Filmen som går, hvor i den (sekunder) akkurat nå, og om den går. {@code nil} eller tom id: lerretet er av. */
    public static void vis(String id, double sekunder, boolean gaar) {
        Kino.SPILLET.vis(id == null || id.isEmpty() ? null : id, sekunder, gaar);
    }

    /** En tekst på lerretet i stedet for filmen; tom tekst tar filmen tilbake. */
    public static void tekst(String linje) {
        Kino.SPILLET.tekst(linje);
    }

    /** Lyset (1 = natt og klart bilde, lavt = blekt om dagen) og fokuset (1 = skarpt), begge 0..1. */
    public static void bilde(double lys, double fokus) {
        Kino.SPILLET.bilde(lys, fokus);
    }

    /**
     * Filmen i vinduet «Watch», hvert bilde fra vinduets {@code render()}: flatens absolutte x/y og størrelse i punkter.
     * 4:3 med sorte felt, alltid klart bilde, tekst midt i når det står en, og sort uten film.
     */
    public static void tegnVindu(double x, double y, double b, double h) {
        Kino.SPILLET.tegnVindu(x, y, b, h);
    }

    /** Volumet 0..1 akkurat nå – kalles hvert sekund. 0 er stille. */
    public static void lyd(double volum) {
        Kino.SPILLET.lyd(volum);
    }
}
