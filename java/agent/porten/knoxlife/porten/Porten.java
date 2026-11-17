package knoxlife.porten;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import knoxlife.agent.KnoxLife;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Set;
import java.util.TreeSet;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import knoxlife.agent.mixin.Start;
import org.objectweb.asm.AnnotationVisitor;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;

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
        // Hver klasse en Java-mod skal inn i, må finnes og ha fått den. En klasse som er borte fra spillet, gir ingen
        // feil fra Mixin – den lastes bare aldri – så det sjekkes her.
        Set<String> mål = mål(laster);
        Set<String> endret = KnoxLife.endret();
        for (String klasse : mål) {
            if (!endret.contains(klasse)) {
                KnoxLife.feil("porten", new IllegalStateException(
                        klasse + " got no Java mod – the class is gone from the game or was never loaded"));
            }
        }
        System.out.println("porten: " + mål.size() + " klasser i spillet skal ha Java-mods, " + endret.size()
                + " fikk dem");
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

    /** Klassene Java-modene skal inn i: {@code @Mixin} på hver klasse i oppsettene, lest med ASM uten å laste dem. */
    static Set<String> mål(ClassLoader laster) throws IOException {
        Set<String> ut = new TreeSet<>();
        for (String oppsett : List.of(Start.OPPSETT, Start.SERVERENS_OPPSETT)) {
            String json = les(laster, oppsett);
            if (json == null) {
                continue;
            }
            Matcher pakke = Pattern.compile("\"package\"\\s*:\\s*\"([^\"]+)\"").matcher(json);
            Matcher liste = Pattern.compile("\"mixins\"\\s*:\\s*\\[([^\\]]*)\\]").matcher(json);
            if (!pakke.find() || !liste.find()) {
                continue;
            }
            Matcher navn = Pattern.compile("\"([^\"]+)\"").matcher(liste.group(1));
            while (navn.find()) {
                String sti = (pakke.group(1) + "." + navn.group(1)).replace('.', '/') + ".class";
                try (InputStream inn = laster.getResourceAsStream(sti)) {
                    if (inn == null) {
                        throw new IOException("the Java mod " + sti + " is missing from the agent");
                    }
                    new ClassReader(inn).accept(new ClassVisitor(Opcodes.ASM9) {
                        @Override
                        public AnnotationVisitor visitAnnotation(String type, boolean synlig) {
                            if (!type.equals("Lorg/spongepowered/asm/mixin/Mixin;")) {
                                return null;
                            }
                            return new AnnotationVisitor(Opcodes.ASM9) {
                                @Override
                                public AnnotationVisitor visitArray(String felt) {
                                    return new AnnotationVisitor(Opcodes.ASM9) {
                                        @Override
                                        public void visit(String n, Object verdi) {
                                            if (verdi instanceof Type t) {
                                                ut.add(t.getClassName());
                                            } else if (verdi instanceof String s) {
                                                ut.add(s.replace('/', '.'));
                                            }
                                        }
                                    };
                                }
                            };
                        }
                    }, ClassReader.SKIP_CODE);
                }
            }
        }
        return ut;
    }

    private static String les(ClassLoader laster, String ressurs) throws IOException {
        try (InputStream inn = laster.getResourceAsStream(ressurs)) {
            return inn == null ? null : new String(inn.readAllBytes(), StandardCharsets.UTF_8);
        }
    }
}
