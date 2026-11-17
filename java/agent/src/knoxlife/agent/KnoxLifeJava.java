package knoxlife.agent;

/**
 * Det Lua i modet ser av Java-delene, som den globale {@code KnoxLifeJava} ({@code shared/KnoxLife/Java.lua} i modet
 * spør alltid gjennom {@code KnoxLife.Java}). Bare det som leser – ingenting her bestemmer over noe, så en annen mod som
 * kaller det, får ingenting ut av det.
 *
 * <p>Tallene er {@code double}, slik Lua har dem.
 */
public final class KnoxLifeJava {
    private KnoxLifeJava() {}

    public static boolean har(String navn) {
        return KnoxLife.har(navn);
    }

    public static double bygg() {
        return KnoxLife.bygg();
    }

    /** Den neste linjen til loggen, eller {@code nil} i Lua. Bare serverdelen legger noe i køen. */
    public static String nesteHendelse() {
        return KnoxLife.nesteHendelse();
    }
}
