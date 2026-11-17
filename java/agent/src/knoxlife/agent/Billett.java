package knoxlife.agent;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.time.Instant;

/**
 * Billetten launcheren legger i {@code Zomboid/KnoxLife/reise.txt}: serverens adresse og en engangsbillett fra tjeneren.
 * Agenten leser den én gang ved oppstart og sletter den, så den aldri brukes to ganger.
 */
public record Billett(String adresse, int port, String billett) {
    /** Billetten er gyldig så lenge (tjeneren sjekker det samme). */
    public static final Duration GYLDIG = Duration.ofMinutes(10);

    /** Leser teksten. {@code null} når noe mangler eller er feil. */
    public static Billett tolk(String tekst) {
        String adresse = null;
        String billett = null;
        int port = -1;
        for (String linje : tekst.split("\n")) {
            String l = linje.strip();
            int lik = l.indexOf('=');
            if (lik <= 0) {
                continue;
            }
            String nøkkel = l.substring(0, lik);
            String verdi = l.substring(lik + 1);
            switch (nøkkel) {
                case "adresse" -> adresse = verdi;
                case "billett" -> billett = verdi;
                case "port" -> {
                    try {
                        port = Integer.parseInt(verdi);
                    } catch (NumberFormatException e) {
                        port = -1;
                    }
                }
                default -> { }
            }
        }
        if (adresse == null || adresse.isEmpty() || billett == null || billett.isEmpty() || port < 1 || port > 65535) {
            return null;
        }
        return new Billett(adresse, port, billett);
    }

    /** Henter billetten og sletter filen. En billett eldre enn {@link #GYLDIG} slettes uten å brukes. */
    public static Billett hent(Path fil, Instant nå) {
        try {
            if (!Files.isRegularFile(fil)) {
                return null;
            }
            Instant laget = Files.getLastModifiedTime(fil).toInstant();
            String tekst = Files.readString(fil, StandardCharsets.US_ASCII);
            Files.deleteIfExists(fil);
            if (laget.plus(GYLDIG).isBefore(nå)) {
                return null;
            }
            return tolk(tekst);
        } catch (IOException | RuntimeException e) {
            return null;
        }
    }

    @Override
    public String toString() {
        // Billetten selv skrives aldri i en logg.
        return "Billett[" + adresse + ":" + port + "]";
    }
}
