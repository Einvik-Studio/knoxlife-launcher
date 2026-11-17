package knoxlife.agent;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * Stempelet launcheren venter på: agenten skriver at den er lastet, med byggenummeret. Gikk noe galt, står det
 * {@code feil=} og ingen {@code bygg=}, så launcheren aldri tror patchen er lastet når den ikke er det.
 */
public final class Stempel {
    private Stempel() {}

    public static void ok(Path fil, long bygg, long tid) throws IOException {
        skriv(fil, "bygg=" + bygg + "\ntid=" + tid + "\n");
    }

    public static void feil(Path fil, Throwable t, long tid) {
        String hva = (t.getClass().getSimpleName() + ": " + t.getMessage()).replaceAll("[\\r\\n]", " ");
        StringBuilder ren = new StringBuilder();
        for (char c : hva.toCharArray()) {
            ren.append(c >= 0x20 && c < 0x7f ? c : '?');
        }
        try {
            skriv(fil, "feil=" + (ren.length() > 300 ? ren.substring(0, 300) : ren) + "\ntid=" + tid + "\n");
        } catch (IOException ignorert) {
            // Ingenting å gjøre: launcheren ser at stempelet ikke kom.
        }
    }

    private static void skriv(Path fil, String tekst) throws IOException {
        Files.createDirectories(fil.toAbsolutePath().getParent());
        Path tmp = fil.resolveSibling(fil.getFileName() + ".tmp");
        Files.writeString(tmp, tekst, StandardCharsets.US_ASCII);
        Files.move(tmp, fil, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
    }
}
