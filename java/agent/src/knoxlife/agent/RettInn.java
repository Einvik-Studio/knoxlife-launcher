package knoxlife.agent;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.time.Instant;

/**
 * «Play» rett inn (launcher.md 3.7 punkt 3): launcheren skriver {@code server=hoved} i {@code Zomboid/KnoxLife/reise.txt}
 * like før den starter spillet. Agenten leser filen én gang ved oppstart og sletter den, og spillet kobler seg til
 * serveren med det navnet i launcherens liste ({@link Servere}) fra hovedmenyen – aldri til en adresse fra filen selv.
 */
public final class RettInn {
    /** Eldre enn dette brukes ikke: Steam kan oppdatere spillet før det starter, men en fil fra i går er glemt. */
    public static final Duration GYLDIG = Duration.ofMinutes(30);
    public static final String FIL = "reise.txt";

    private RettInn() {}

    /** Navnet på serveren i teksten, eller {@code null} når den mangler eller er feil. */
    public static String tolk(String tekst) {
        for (String linje : tekst.split("\n")) {
            String l = linje.strip();
            if (l.startsWith("server=")) {
                String navn = l.substring("server=".length());
                return navn.matches("[a-z]{1,20}") ? navn : null;
            }
        }
        return null;
    }

    /** Henter navnet og sletter filen. En fil eldre enn {@link #GYLDIG} slettes uten å brukes. */
    public static String hent(Path fil, Instant nå) {
        try {
            if (!Files.isRegularFile(fil)) {
                return null;
            }
            Instant laget = Files.getLastModifiedTime(fil).toInstant();
            String tekst = Files.size(fil) > 4096 ? "" : Files.readString(fil, StandardCharsets.US_ASCII);
            Files.deleteIfExists(fil);
            if (laget.plus(GYLDIG).isBefore(nå)) {
                return null;
            }
            return tolk(tekst);
        } catch (IOException | RuntimeException e) {
            return null;
        }
    }
}
