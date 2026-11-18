package knoxlife.agent;

/**
 * Det Lua i modet ser av Java-delene, som den globale {@code KnoxLifeJava} ({@code shared/KnoxLife/Java.lua} i modet
 * spør alltid gjennom {@code KnoxLife.Java}). Nesten bare det som leser – ingenting her bestemmer over noe, så en annen
 * mod som kaller det, får ingenting ut av det. De to som skriver ({@link #paaKnoxLife()} og {@link #omstart(boolean)}), sier
 * bare at spilleren selv skal kobles til den samme serveren igjen etter et brudd (J6), og {@link #reise(String)} sender
 * spilleren til en av Knox Lifes egne servere – bare dem launcheren kjenner (J5). Hos spilleren kan modet også slå
 * profileren på ({@link #profil(boolean)}, J7) og gi listen over hendelser som hoppes over ({@link #hopp(String)}, J9) –
 * en annen mod kunne like gjerne fjernet de samme funksjonene selv, så det gir ingen fordel. På serveren er begge låst.
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

    /**
     * J7 hos spilleren (launcher.md 3.8 punkt 1): profileren på ({@code true}) eller av, når staff har bedt om det. På
     * serveren står den alltid på, og svaret er {@code false}.
     */
    public static boolean profil(boolean på) {
        if (Profil.SPILLET.låst()) {
            return false;
        }
        Profil.SPILLET.på(på, System.currentTimeMillis());
        return true;
    }

    /**
     * Tiden siden sist som én linje JSON (de dyreste funksjonene, tikkene), og tellingen begynner på nytt – eller
     * {@code nil} når profileren står av. På serveren henter serverdelen minuttene selv, og svaret er {@code nil}.
     */
    public static String profilMinutt() {
        if (Profil.SPILLET.låst() || !Profil.SPILLET.på()) {
            return null;
        }
        return Profil.SPILLET.minutt(System.currentTimeMillis()).json();
    }

    /**
     * J9 hos spilleren (punkt 4): listen over hendelser som hoppes over, én linje per hendelse
     * ({@code <hendelse> <mod> <fil>:<linje> # <grunn>}), slik serveren har den. Gir hvor mange linjer som ble tatt med;
     * på serveren gjelder bare serverens {@code java.ini}, og svaret er -1.
     */
    public static double hopp(String linjer) {
        if (Profil.SPILLET.låst()) {
            return -1;
        }
        java.util.List<Profil.Hopp> l = new java.util.ArrayList<>();
        if (linjer != null) {
            for (String linje : linjer.split("\\R")) {
                Profil.Hopp h = Profil.Hopp.tolk(linje);
                if (h != null) {
                    l.add(h);
                }
            }
        }
        return Profil.SPILLET.hopp(l).size();
    }

    /** Listen over hendelser som hoppes over, slik den gjelder her – serveren gir den videre til spillerne. */
    public static String hoppListe() {
        StringBuilder b = new StringBuilder();
        for (Profil.Hopp h : Profil.SPILLET.hopp()) {
            b.append(h.somLinje()).append('\n');
        }
        return b.toString();
    }

    /** Den neste linjen til loggen, eller {@code nil} i Lua. Bare serverdelen legger noe i køen. */
    public static String nesteHendelse() {
        return KnoxLife.nesteHendelse();
    }
}
