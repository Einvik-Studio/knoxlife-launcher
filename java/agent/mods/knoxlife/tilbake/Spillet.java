package knoxlife.tilbake;

import knoxlife.agent.Tilbake;
import se.krka.kahlua.vm.KahluaTable;
import zombie.GameWindow;
import zombie.Lua.LuaManager;
import zombie.core.Core;
import zombie.network.GameClient;
import zombie.ui.UIElement;

/**
 * J6 i spillet (launcher.md 3.6 punkt 7): leser det {@link Tilbake} trenger, og gjør det den svarer. Kalles av
 * {@code TilbakeSkjermMixin} (skjermen ved bruddet) og {@code TilbakeMenyMixin} (hovedmenyen), alltid fra spillets
 * egen tråd.
 *
 * <p>Den nye tilkoblingen går gjennom spillets egen skjerm for tilkobling ({@code ConnectToServer:connect} i
 * {@code client/OptionScreens/ConnectToServer.lua}), akkurat som når spilleren trykker «Join»: spillerne ser den
 * vanlige skjermen med «Abort», og spillets egne beskjeder (nedlasting av mods, køen) står som før.
 *
 * <p>J5 (launcher.md 3.6 punkt 3): {@link Tilbake#fraSpillet()} får innloggingen spillet har nå herfra, når modet sender
 * spilleren til testserveren – den samme veien gjennom hovedmenyen tar spillet dit.
 *
 * <p>Feiler noe her, stopper J6 for denne gangen og sier fra i konsollen – spillet går videre som uten Java-moden.
 */
public final class Spillet {
    private static boolean feilet;

    static {
        Tilbake.leser(Spillet::innlogging);
    }

    private Spillet() {}

    /** Innloggingen spillet har i minnet ({@code GameClient}), eller {@code null} når det ikke er koblet til noe. */
    static Tilbake.Innlogging innlogging() {
        if (GameClient.username == null || GameClient.ip == null || GameClient.ip.isBlank()) {
            return null;
        }
        return new Tilbake.Innlogging(GameClient.username, tom(GameClient.password), GameClient.ip,
                tom(GameClient.localIP), String.valueOf(GameClient.port), tom(GameClient.serverPassword),
                tom(GameClient.serverName), GameClient.useSteamRelay, GameClient.authType);
    }

    private static long nå() {
        return System.nanoTime() / 1_000_000L;
    }

    private static String tom(String s) {
        return s == null ? "" : s;
    }

    /** {@code ServerDisconnectState.enter}, før spillet viser skjermen. */
    public static void vedBrudd() {
        try {
            if (Tilbake.SPILLET.brudd(nå(), GameWindow.kickReason, innlogging())) {
                // Teksten midt på skjermen er grunnen spillet viser (ServerDisconnectState.render).
                GameWindow.kickReason = Tilbake.SPILLET.skjermtekst();
            }
        } catch (RuntimeException | LinkageError e) {
            stopp("ved bruddet", e);
        }
    }

    /** {@code ServerDisconnectState.update}, hvert bilde: etter 3 sekunder går spillet til hovedmenyen. */
    public static void påSkjermen() {
        try {
            if (Tilbake.SPILLET.tilMenyen(nå())) {
                Core.getInstance().exitToMenu();
            }
        } catch (RuntimeException | LinkageError e) {
            stopp("på skjermen", e);
        }
    }

    /** {@code MainScreenState.update}, hvert bilde i hovedmenyen. */
    public static void iMenyen() {
        if (!Tilbake.SPILLET.iGang()) {
            return;
        }
        try {
            KahluaTable env = LuaManager.env;
            KahluaTable klasse = env == null ? null : tabell(env.rawget("ConnectToServer"));
            KahluaTable skjerm = klasse == null ? null : tabell(klasse.rawget("instance"));
            KahluaTable hoved = env == null ? null : tabell(env.rawget("MainScreen"));
            KahluaTable meny = hoved == null ? null : tabell(hoved.rawget("instance"));
            KahluaTable bunn = meny == null ? null : tabell(meny.rawget("bottomPanel"));
            if (skjerm == null || bunn == null) {
                return; // Menyen er ikke laget ennå (spillet laster Lua på nytt etter bruddet).
            }
            var m = new Tilbake.Meny(synlig(bunn), Boolean.TRUE.equals(skjerm.rawget("connecting")), synlig(skjerm),
                    GameClient.connection != null);
            Tilbake.Steg steg = Tilbake.SPILLET.iMenyen(nå(), m);
            switch (steg.handling()) {
                case KOBLE_TIL -> {
                    kobleTil(klasse, skjerm, bunn);
                    tekst(skjerm, steg.tekst());
                }
                case VIS_TEKST -> tekst(skjerm, steg.tekst());
                default -> {
                }
            }
        } catch (RuntimeException | LinkageError e) {
            stopp("i hovedmenyen", e);
        }
    }

    private static void kobleTil(KahluaTable klasse, KahluaTable skjerm, KahluaTable bunn) {
        Tilbake.Innlogging inn = Tilbake.SPILLET.innlogging();
        Object connect = klasse.rawget("connect");
        if (inn == null || connect == null) {
            throw new IllegalStateException("ConnectToServer.connect mangler");
        }
        // Som «Back» etter et forsøk som feilet: spillet slipper den gamle tilkoblingen før den neste (gjør ingenting
        // når det ikke er noen).
        LuaManager.GlobalObject.backToSinglePlayer();
        // ConnectToServer:connect(previousScreen, serverName, userName, password, IP, localIP, port, serverPassword,
        // useSteamRelay, doHash, authType). Passordet er alt gjort om av spillet, så doHash er false. Tall er double
        // i Lua.
        Object[] svar = LuaManager.caller.pcall(LuaManager.thread, connect, skjerm, bunn, inn.servernavn(),
                inn.bruker(), inn.passord(), inn.ip(), inn.lokalIp(), inn.port(), inn.serverpassord(),
                inn.steamRelay(), Boolean.FALSE, (double) inn.autentisering());
        if (svar == null || svar.length == 0 || !Boolean.TRUE.equals(svar[0])) {
            String grunn = svar != null && svar.length > 1 ? String.valueOf(svar[1]) : "ingen svar";
            throw new IllegalStateException("ConnectToServer.connect feilet: "
                    + grunn.substring(0, Math.min(200, grunn.length())));
        }
    }

    private static void tekst(KahluaTable skjerm, String tekst) {
        KahluaTable etikett = tabell(skjerm.rawget("connectLabel"));
        if (etikett != null && tekst != null) {
            etikett.rawset("name", tekst);
        }
    }

    private static KahluaTable tabell(Object o) {
        return o instanceof KahluaTable t ? t : null;
    }

    private static boolean synlig(KahluaTable element) {
        return element.rawget("javaObject") instanceof UIElement e && Boolean.TRUE.equals(e.isVisible());
    }

    private static void stopp(String hvor, Throwable e) {
        Tilbake.SPILLET.stopp();
        if (!feilet) {
            feilet = true;
            // Ikke KnoxLife.feil: den sier fra i rapporten til juksesperren, og dette er ingen feil ved Java-modene.
            System.err.println("[KnoxLife] J6 " + hvor + ": " + e);
        }
    }
}
