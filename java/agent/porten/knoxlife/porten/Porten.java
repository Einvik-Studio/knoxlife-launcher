package knoxlife.porten;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import knoxlife.agent.KnoxLife;

/**
 * Porten i bygget: starter med agenten mot spillets ekte {@code projectzomboid.jar} og laster hver klasse i den (uten å
 * kjøre noe). Hver Java-mod legges da inn akkurat som i spillet. Kan én av dem ikke legges inn – spillet har endret seg –
 * stopper bygget, i stedet for at en spiller merker det etter flere uker.
 *
 * <p>Kjøres av {@code bygg.sh} når {@code SPILLETS_MAPPE} er satt. Spillets jar ligger aldri i repoet.
 */
public final class Porten {
    private Porten() {}

    public static void main(String[] args) throws IOException {
        Path jar = Path.of(args[0]);
        List<String> klasser = new ArrayList<>();
        try (JarFile fil = new JarFile(jar.toFile())) {
            for (JarEntry e : Collections.list(fil.entries())) {
                String n = e.getName();
                if (n.endsWith(".class") && !n.equals("module-info.class") && !n.startsWith("META-INF/")) {
                    klasser.add(n.substring(0, n.length() - ".class".length()).replace('/', '.'));
                }
            }
        }
        ClassLoader laster = ClassLoader.getSystemClassLoader();
        int lastet = 0;
        List<String> mangler = new ArrayList<>();
        for (String k : klasser) {
            try {
                Class.forName(k, false, laster);
                lastet++;
            } catch (LinkageError | ClassNotFoundException e) {
                // Klasser som peker på biblioteker serveren ikke har (for eksempel bare for klienten), lastes ikke her.
                // Det er spillets sak, ikke vår – men det telles, så en stor endring synes.
                mangler.add(k + ": " + e);
            }
        }
        System.out.println("porten: " + lastet + " av " + klasser.size() + " klasser lastet, " + mangler.size()
                + " pekte på noe som mangler her");
        for (String m : mangler.subList(0, Math.min(5, mangler.size()))) {
            System.out.println("   " + m);
        }
        List<String> feil = KnoxLife.feil();
        if (!feil.isEmpty()) {
            System.out.println("porten: " + feil.size() + " Java-mods kunne ikke legges inn:");
            feil.forEach(f -> System.out.println("   " + f));
            System.exit(1);
        }
        if (lastet == 0) {
            System.out.println("porten: ingen klasser lastet – feil jar?");
            System.exit(1);
        }
        System.out.println("porten: alle Java-modene ble lagt inn (" + KnoxLife.rapport() + ")");
    }
}
