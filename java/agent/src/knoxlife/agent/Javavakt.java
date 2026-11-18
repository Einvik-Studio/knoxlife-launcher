package knoxlife.agent;

import java.io.IOException;
import java.lang.management.ManagementFactory;
import java.nio.file.FileVisitResult;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.nio.file.SimpleFileVisitor;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

/**
 * Juksesperren hos spilleren: ser gjennom Java-en spillet kjører i og lister det som ikke hører hjemme der.
 *
 * <p>Vakten avgjør ingenting selv. Funnene legges på innloggingen, og serveren avgjør. Funnene sier hva, aldri hvor:
 * bare filnavnet, aldri hele stien, så ingenting om spillerens mapper eller brukernavn sendes.
 */
public final class Javavakt {
    public static final int FLEST_FUNN = 8;
    public static final int LENGSTE_FUNN = 96;
    public static final String SPILLETS_JAR = "projectzomboid.jar";
    /** Miljøvariablene Java leser valg fra uten at de står i spillets fil. */
    static final List<String> MILJØ = List.of("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS");
    /** Flest filer som gjennomgås i en mappe på klassestien. */
    static final int FLEST_FILER = 300_000;

    private Javavakt() {}

    /** Det vakten ser på, samlet ett sted, så reglene kan prøves uten en ekte Java. */
    public record Inntrykk(
            List<String> jvmValg,
            List<String> klassesti,
            Map<String, String> miljø,
            String systemlaster,
            Boolean dynamiskeAgenter,
            Path vårAgent) {

        /** Slik Java-en spillet kjører i, ser ut nå. */
        public static Inntrykk herfra(Path vårAgent) {
            String sti = System.getProperty("java.class.path", "");
            List<String> klassesti = sti.isEmpty() ? List.of() : Arrays.asList(sti.split(java.io.File.pathSeparator, -1));
            return new Inntrykk(
                    ManagementFactory.getRuntimeMXBean().getInputArguments(),
                    klassesti,
                    System.getenv(),
                    ClassLoader.getSystemClassLoader().getClass().getName(),
                    dynamiskeAgenterHerfra(),
                    vårAgent);
        }
    }

    /** Om Java slipper inn agenter etter at spillet har startet (et verktøy kan da koble seg på underveis). */
    static Boolean dynamiskeAgenterHerfra() {
        try {
            var hs = ManagementFactory.getPlatformMXBean(com.sun.management.HotSpotDiagnosticMXBean.class);
            return Boolean.valueOf(hs.getVMOption("EnableDynamicAgentLoading").getValue());
        } catch (Throwable t) {
            return null; // ikke HotSpot, eller valget finnes ikke i denne Java-versjonen
        }
    }

    public static List<String> funn(Inntrykk i) {
        List<String> ut = new ArrayList<>();
        List<String> valg = i.jvmValg();
        for (int n = 0; n < valg.size(); n++) {
            String v = valg.get(n);
            if (v.startsWith("-javaagent:")) {
                String sti = v.substring("-javaagent:".length());
                int lik = sti.indexOf('=');
                String jar = lik >= 0 ? sti.substring(0, lik) : sti;
                if (!erVår(jar, i.vårAgent())) {
                    legg(ut, "javaagent " + blad(jar));
                }
            } else if (v.startsWith("-agentlib:jdwp") || v.equals("-Xdebug") || v.startsWith("-Xrunjdwp")) {
                legg(ut, "debugger");
            } else if (v.startsWith("-agentlib:") || v.startsWith("-agentpath:")) {
                String rest = v.substring(v.indexOf(':') + 1);
                int lik = rest.indexOf('=');
                legg(ut, "native agent " + blad(lik >= 0 ? rest.substring(0, lik) : rest));
            } else if (v.startsWith("-Xbootclasspath")) {
                legg(ut, "boot class path");
            } else if (v.startsWith("--patch-module")) {
                legg(ut, "patch-module");
            } else if (v.startsWith("-Djava.system.class.loader")) {
                legg(ut, "custom class loader");
            } else if (v.equals("-XX:+EnableDynamicAgentLoading")) {
                legg(ut, "dynamic agents allowed");
            }
        }
        if (Boolean.TRUE.equals(i.dynamiskeAgenter()) && !ut.contains("dynamic agents allowed")) {
            legg(ut, "dynamic agents allowed");
        }
        for (String navn : MILJØ) {
            String v = i.miljø().get(navn);
            if (v != null && !v.isBlank()) {
                legg(ut, navn + " " + utenStier(v.trim()));
            }
        }
        if (i.systemlaster() != null && !i.systemlaster().startsWith("jdk.internal.loader.")) {
            legg(ut, "class loader " + i.systemlaster());
        }
        boolean sattSpillet = false;
        for (String post : i.klassesti()) {
            if (post.isEmpty()) {
                legg(ut, "class path: working folder");
                continue;
            }
            if (post.endsWith("*")) {
                legg(ut, "class path wildcard " + blad(post.substring(0, post.length() - 1)));
                continue;
            }
            if (erVår(post, i.vårAgent())) {
                continue;
            }
            if (blad(post).equalsIgnoreCase(SPILLETS_JAR) && !sattSpillet) {
                sattSpillet = true;
                continue;
            }
            Path p;
            try {
                p = Path.of(post);
            } catch (InvalidPathException e) {
                legg(ut, "class path " + blad(post));
                continue;
            }
            if (Files.isDirectory(p)) {
                String klasse = førsteKlasse(p);
                if (klasse != null) {
                    legg(ut, "classes in folder " + blad(post) + ": " + klasse);
                }
            } else {
                legg(ut, "class path " + blad(post));
            }
        }
        return ut;
    }

    /** Peker stien på vår egen agent? Sammenlignes som filer, så korte 8.3-navn og store og små bokstaver er det samme. */
    static boolean erVår(String sti, Path vår) {
        if (vår == null) {
            return false;
        }
        try {
            Path p = Path.of(sti);
            if (Files.exists(p) && Files.exists(vår)) {
                return Files.isSameFile(p, vår);
            }
            return p.toAbsolutePath().normalize().equals(vår.toAbsolutePath().normalize());
        } catch (InvalidPathException | IOException e) {
            return false;
        }
    }

    /**
     * Den første klassefilen Java kan laste fra en mappe, eller {@code null}.
     *
     * <p>Bare der Java kan finne en klasse: hver mappe på veien må kunne være en del av et pakkenavn, og filen et
     * klassenavn. Andre mapper hoppes over med alt i dem – som Workshop-mappen {@code steamapps/workshop/content/108600}
     * i spillmappen, der mods kan ha med klassefiler spillet aldri laster. Java-kode kan ikke nevne en klasse der, og
     * {@code ServiceLoader} nekter slike navn.
     */
    static String førsteKlasse(Path mappe) {
        String[] funnet = {null};
        int[] talt = {0};
        try {
            Files.walkFileTree(mappe, new SimpleFileVisitor<>() {
                @Override
                public FileVisitResult preVisitDirectory(Path d, BasicFileAttributes a) {
                    if (++talt[0] > FLEST_FILER) {
                        funnet[0] = FOR_MANGE;
                        return FileVisitResult.TERMINATE;
                    }
                    return d.equals(mappe) || javaNavn(navn(d)) ? FileVisitResult.CONTINUE : FileVisitResult.SKIP_SUBTREE;
                }

                @Override
                public FileVisitResult visitFile(Path f, BasicFileAttributes a) {
                    if (++talt[0] > FLEST_FILER) {
                        funnet[0] = FOR_MANGE;
                        return FileVisitResult.TERMINATE;
                    }
                    String n = navn(f);
                    if (n.endsWith(".class") && javaNavn(n.substring(0, n.length() - ".class".length()))) {
                        funnet[0] = mappe.relativize(f).toString().replace('\\', '/');
                        return FileVisitResult.TERMINATE;
                    }
                    return FileVisitResult.CONTINUE;
                }

                @Override
                public FileVisitResult visitFileFailed(Path f, IOException e) {
                    return FileVisitResult.CONTINUE; // en fil eller mappe som ikke kan leses, kan ikke Java laste fra heller
                }
            });
            return funnet[0];
        } catch (IOException | RuntimeException e) {
            return null;
        }
    }

    /** Funnet når en mappe har flere filer enn vakten går gjennom. */
    static final String FOR_MANGE = "(not checked, too many files)";

    private static String navn(Path p) {
        Path n = p.getFileName();
        return n == null ? "" : n.toString();
    }

    /** Kan teksten være en del av et pakkenavn eller et klassenavn i Java ({@code zombie}, {@code GameClient$1})? */
    static boolean javaNavn(String s) {
        if (s.isEmpty() || !Character.isJavaIdentifierStart(s.codePointAt(0))) {
            return false;
        }
        for (int i = Character.charCount(s.codePointAt(0)); i < s.length(); ) {
            int c = s.codePointAt(i);
            if (!Character.isJavaIdentifierPart(c)) {
                return false;
            }
            i += Character.charCount(c);
        }
        return true;
    }

    /** Filnavnet til slutt i en sti, uten mappene foran. */
    static String blad(String sti) {
        String s = sti.trim();
        while (s.endsWith("/") || s.endsWith("\\")) {
            s = s.substring(0, s.length() - 1);
        }
        int i = Math.max(s.lastIndexOf('/'), s.lastIndexOf('\\'));
        String b = i >= 0 ? s.substring(i + 1) : s;
        return b.isEmpty() ? "." : b;
    }

    /** Hvert ord som ser ut som en sti, byttes med filnavnet sitt. */
    static String utenStier(String tekst) {
        StringBuilder ut = new StringBuilder();
        for (String ord : tekst.split("\\s+")) {
            if (ut.length() > 0) {
                ut.append(' ');
            }
            int kolon = ord.indexOf(':');
            if ((ord.contains("/") || ord.contains("\\")) && kolon > 0 && ord.startsWith("-")) {
                ut.append(ord, 0, kolon + 1).append(blad(ord.substring(kolon + 1)));
            } else if (ord.contains("/") || ord.contains("\\")) {
                ut.append(blad(ord));
            } else {
                ut.append(ord);
            }
        }
        return ut.toString();
    }

    private static void legg(List<String> ut, String funn) {
        if (ut.size() >= FLEST_FUNN) {
            return;
        }
        StringBuilder s = new StringBuilder();
        for (char c : funn.toCharArray()) {
            s.append(c >= 0x20 && c < 0x7f && c != '|' ? c : '?');
        }
        String ren = s.length() > LENGSTE_FUNN ? s.substring(0, LENGSTE_FUNN) : s.toString();
        if (!ut.contains(ren)) {
            ut.add(ren);
        }
    }
}
