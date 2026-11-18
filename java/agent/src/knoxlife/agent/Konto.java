package knoxlife.agent;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.regex.Pattern;

/**
 * Brukernavnet spilleren har på Knox Life, husket i {@code Zomboid/KnoxLife/konto.txt} etter hver innlogging der
 * (launcher.md 3.7 punkt 3). «Play» rett inn logger inn med det. Første gang på en ny PC er det ikke der: da logger
 * agenten inn med {@link #UKJENT}, og serveren svarer med brukernavnet Steam-kontoen har.
 */
public final class Konto {
    public static final String FIL = "konto.txt";
    /**
     * Brukernavnet når agenten ikke vet kontoen ennå. Mellomrommet gjør at det aldri kan være et ekte brukernavn
     * (serveren sier det samme).
     */
    public static final String UKJENT = "Knox Life";
    /** Slik et brukernavn på Knox Life ser ut (tjeneren lager bare slike). */
    private static final Pattern GYLDIG = Pattern.compile("[A-Za-z0-9_.-]{3,20}");

    private Konto() {}

    public static boolean gyldig(String navn) {
        return navn != null && GYLDIG.matcher(navn).matches();
    }

    /** Brukernavnet i filen, eller {@link #UKJENT} når filen mangler eller er feil. */
    public static String les(Path mappe) {
        if (mappe == null) {
            return UKJENT;
        }
        try {
            Path fil = mappe.resolve(FIL);
            if (!Files.isRegularFile(fil) || Files.size(fil) > 256) {
                return UKJENT;
            }
            String navn = Files.readString(fil, StandardCharsets.US_ASCII).strip();
            return gyldig(navn) ? navn : UKJENT;
        } catch (IOException | RuntimeException e) {
            return UKJENT;
        }
    }

    /** Husker brukernavnet. {@code false} når det ikke er et brukernavn fra Knox Life, eller filen ikke kunne skrives. */
    public static boolean skriv(Path mappe, String navn) {
        if (mappe == null || !gyldig(navn)) {
            return false;
        }
        try {
            if (navn.equals(les(mappe))) {
                return true;
            }
            Files.createDirectories(mappe);
            Path tmp = mappe.resolve(FIL + ".tmp");
            Files.writeString(tmp, navn + "\n", StandardCharsets.US_ASCII);
            Files.move(tmp, mappe.resolve(FIL), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
            return true;
        } catch (IOException | RuntimeException e) {
            return false;
        }
    }
}
