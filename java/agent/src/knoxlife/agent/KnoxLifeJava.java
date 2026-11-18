package knoxlife.agent;

/**
 * Det Lua i modet ser av Java-delene, som den globale {@code KnoxLifeJava} ({@code shared/KnoxLife/Java.lua} i modet
 * spør alltid gjennom {@code KnoxLife.Java}). Nesten bare det som leser – ingenting her bestemmer over noe, så en annen
 * mod som kaller det, får ingenting ut av det. De to som skriver ({@link #paaKnoxLife()} og {@link #omstart(boolean)}), sier
 * bare at spilleren selv skal kobles til den samme serveren igjen etter et brudd (J6), og {@link #reise(String)} sender
 * spilleren til en av Knox Lifes egne servere – bare dem launcheren kjenner (J5).
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

    /**
     * Modet er i spillet på Knox Life (klienten, ved hver innlogging): faller forbindelsen uten at spilleren er kastet
     * ut, kobler spillet seg til igjen selv ({@link Tilbake}). ASCII i navnet, så Lua kan skrive det.
     */
    public static void paaKnoxLife() {
        Tilbake.SPILLET.påKnoxLife();
    }

    /**
     * Serveren har sagt fra at den starter på nytt ({@code true}): skjermen ved bruddet sier det, og spillet kobler seg
     * til igjen. {@code false}: omstarten er avlyst.
     */
    public static void omstart(boolean kommer) {
        Tilbake.SPILLET.omstart(kommer);
    }

    /**
     * J5 (launcher.md 3.6 punkt 3): gå til Knox Lifes server med navnet («test») med den samme innloggingen, når spillet
     * er ute i hovedmenyen. Modet kaller det når serveren sier at karakteren er klar der, og sender så spillet til
     * menyen. {@code false}: launcheren kjenner ikke serveren, eller dette er ikke Knox Life – spilleren blir der hen er.
     */
    public static boolean reise(String til) {
        Servere.Server mål = KnoxLife.server(til);
        if (mål == null) {
            return false;
        }
        Tilbake.Innlogging fra = Tilbake.fraSpillet();
        return fra != null && Tilbake.SPILLET.reise(System.nanoTime() / 1_000_000L, fra, mål);
    }

    /** Den neste linjen til loggen, eller {@code nil} i Lua. Bare serverdelen legger noe i køen. */
    public static String nesteHendelse() {
        return KnoxLife.nesteHendelse();
    }
}
