package knoxlife.agent;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Pattern;

/**
 * Knox Lifes servere slik launcheren kjenner dem: {@code Zomboid/KnoxLife/servere.txt}, skrevet av launcheren ved hver
 * start fra det signerte manifestet. Én server per linje, {@code navn=adresse:port} («hoved», «test»). Spillet går bare
 * til servere som står her (J5, launcher.md 3.6 punkt 3) – aldri til en adresse fra serveren eller modet.
 */
public record Servere(Map<String, Server> alle) {
    /** Høyst så mange servere i listen. */
    public static final int FLEST = 8;
    private static final Pattern NAVN = Pattern.compile("[a-z]{1,20}");
    private static final Pattern ADRESSE = Pattern.compile("[A-Za-z0-9.-]{1,253}");

    public record Server(String navn, String adresse, int port) {
        /** Slik spillerne ser serveren i teksten ved overgangen. */
        public String tittel() {
            return switch (navn) {
                case "hoved" -> "Knox Life";
                case "test" -> "the Knox Life test server";
                default -> "Knox Life (" + navn + ")";
            };
        }
    }

    public Servere {
        alle = Map.copyOf(alle);
    }

    /** Leser teksten. Linjer som ikke stemmer, hoppes over; {@code null} når ingen server står der. */
    public static Servere tolk(String tekst) {
        Map<String, Server> alle = new LinkedHashMap<>();
        for (String linje : tekst.split("\n")) {
            String l = linje.strip();
            int lik = l.indexOf('=');
            int kolon = l.lastIndexOf(':');
            if (lik <= 0 || kolon <= lik + 1 || alle.size() >= FLEST) {
                continue;
            }
            String navn = l.substring(0, lik);
            String adresse = l.substring(lik + 1, kolon);
            int port;
            try {
                port = Integer.parseInt(l.substring(kolon + 1));
            } catch (NumberFormatException e) {
                continue;
            }
            if (!NAVN.matcher(navn).matches() || !ADRESSE.matcher(adresse).matches() || port < 1 || port > 65535
                    || alle.containsKey(navn)) {
                continue;
            }
            alle.put(navn, new Server(navn, adresse, port));
        }
        return alle.isEmpty() ? null : new Servere(alle);
    }

    /** Leser filen (den står til launcheren skriver den på nytt). {@code null} når den mangler eller er tom. */
    public static Servere les(Path fil) {
        try {
            if (!Files.isRegularFile(fil) || Files.size(fil) > 4096) {
                return null;
            }
            return tolk(Files.readString(fil, StandardCharsets.US_ASCII));
        } catch (IOException | RuntimeException e) {
            return null;
        }
    }

    /** Serveren med navnet, eller {@code null}. */
    public Server finn(String navn) {
        return navn == null ? null : alle.get(navn);
    }
}
