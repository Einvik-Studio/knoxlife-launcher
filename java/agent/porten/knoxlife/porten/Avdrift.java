package knoxlife.porten;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.TreeSet;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import org.objectweb.asm.AnnotationVisitor;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.FieldInsnNode;
import org.objectweb.asm.tree.FrameNode;
import org.objectweb.asm.tree.IincInsnNode;
import org.objectweb.asm.tree.IntInsnNode;
import org.objectweb.asm.tree.InvokeDynamicInsnNode;
import org.objectweb.asm.tree.JumpInsnNode;
import org.objectweb.asm.tree.LabelNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.LineNumberNode;
import org.objectweb.asm.tree.LookupSwitchInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.MultiANewArrayInsnNode;
import org.objectweb.asm.tree.TableSwitchInsnNode;
import org.objectweb.asm.tree.TryCatchBlockNode;
import org.objectweb.asm.tree.TypeInsnNode;
import org.objectweb.asm.tree.VarInsnNode;

/**
 * Avdriftssjekken (launcher.md 3.9 punkt 3): et fingeravtrykk av hver metode i spillet Java-modene våre griper inn i.
 * En ny versjon av spillet kan endre en metode uten at Mixin merker det – innføringen treffer fortsatt, men gjør noe
 * annet enn før. Derfor sammenlignes koden, og bygget stopper til noen har lest den endrede metoden og sjekket Java-moden.
 *
 * <p>Fingeravtrykket er SHA-1 av instruksjonene skrevet som tekst – uten linjenumrene og rammene, og med merkene
 * nummerert i rekkefølge – så det bare endres når koden i metoden endres, ikke når noe annet i klassen flyttes.
 *
 * <pre>
 *   Avdrift sjekk &lt;fingeravtrykk.txt&gt; &lt;projectzomboid.jar&gt; &lt;mappe med Java-modene&gt;...
 *   Avdrift lag   &lt;fingeravtrykk.txt&gt; &lt;projectzomboid.jar&gt; &lt;mappe med Java-modene&gt;...
 * </pre>
 */
public final class Avdrift {
    private Avdrift() {}

    public static void main(String[] args) throws IOException {
        if (args.length < 4 || !(args[0].equals("sjekk") || args[0].equals("lag"))) {
            System.err.println("bruk: Avdrift sjekk|lag <fingeravtrykk.txt> <projectzomboid.jar> <mappe med Java-modene>...");
            System.exit(2);
        }
        Path fil = Path.of(args[1]);
        List<Path> mapper = new ArrayList<>();
        for (int i = 3; i < args.length; i++) {
            mapper.add(Path.of(args[i]));
        }
        Map<String, Map<String, TreeSet<String>>> mål = mål(mapper);
        Map<String, String> nå = fingeravtrykk(Path.of(args[2]), mål);
        List<String> borte = nå.entrySet().stream().filter(e -> BORTE.equals(e.getValue())).map(Map.Entry::getKey).toList();
        if (args[0].equals("lag")) {
            if (!borte.isEmpty()) {
                borte.forEach(m -> System.out.println("   borte  " + m + hvem(m, mål)));
                System.out.println("avdrift: rett Java-modene over først – filen lages når alle metodene finnes i spillet");
                System.exit(1);
            }
            Files.writeString(fil, tekst(nå), StandardCharsets.UTF_8);
            System.out.println("avdrift: " + nå.size() + " metoder skrevet til " + fil.getFileName());
            return;
        }
        Map<String, String> før = Files.exists(fil) ? les(Files.readString(fil, StandardCharsets.UTF_8)) : Map.of();
        int uendret = 0;
        List<String> funn = new ArrayList<>();
        TreeSet<String> alle = new TreeSet<>(nå.keySet());
        alle.addAll(før.keySet());
        for (String m : alle) {
            String a = før.get(m);
            String b = nå.get(m);
            if (BORTE.equals(b)) {
                funn.add("borte  " + m + hvem(m, mål));
            } else if (a != null && a.equals(b)) {
                uendret++;
            } else if (b == null) {
                if (!dekket(m, nå)) {
                    funn.add("ikke i bruk " + m);
                }
            } else if (a == null) {
                funn.add("ny     " + m + hvem(m, mål));
            } else {
                funn.add("endret " + m + hvem(m, mål));
            }
        }
        System.out.println("avdrift: " + uendret + " uendret, " + funn.size() + " å se på");
        if (funn.isEmpty()) {
            return;
        }
        funn.forEach(f -> System.out.println("   " + f));
        if (!borte.isEmpty()) {
            System.out.println("avdrift: rett Java-modene som griper inn i det som er borte – bygget skriver ut den nye");
            System.out.println("filen når alle metodene finnes i spillet");
        } else {
            System.out.println("avdrift: les metodene over i den nye versjonen av spillet og sjekk Java-modene som griper inn i");
            System.out.println("dem. Når de er i orden, skal " + fil.getFileName() + " være:");
            System.out.println("----");
            System.out.print(tekst(nå));
            System.out.println("----");
        }
        System.exit(1);
    }

    static final String BORTE = "borte";

    /**
     * Klassene og metodene Java-modene griper inn i: {@code @Mixin} på klassen, {@code method} i hver innføring og
     * {@code @Overwrite}. Svaret er klasse → velger (som i Java-moden) → Java-modene som bruker den.
     */
    static Map<String, Map<String, TreeSet<String>>> mål(List<Path> mapper) throws IOException {
        Map<String, Map<String, TreeSet<String>>> ut = new TreeMap<>();
        for (Path mappe : mapper) {
            if (!Files.isDirectory(mappe)) {
                continue;
            }
            List<Path> filer;
            try (Stream<Path> s = Files.walk(mappe)) {
                filer = s.filter(p -> p.toString().endsWith(".class")).sorted().toList();
            }
            for (Path f : filer) {
                List<String> klasser = new ArrayList<>();
                List<String> metoder = new ArrayList<>();
                String[] mod = { "" };
                new ClassReader(Files.readAllBytes(f)).accept(new ClassVisitor(Opcodes.ASM9) {
                    @Override
                    public void visit(int v, int a, String navn, String sig, String over, String[] grensesnitt) {
                        mod[0] = navn.substring(navn.lastIndexOf('/') + 1);
                    }

                    @Override
                    public AnnotationVisitor visitAnnotation(String type, boolean synlig) {
                        if (!type.equals("Lorg/spongepowered/asm/mixin/Mixin;")) {
                            return null;
                        }
                        return verdier(null, klasser);
                    }

                    @Override
                    public MethodVisitor visitMethod(int a, String navn, String desc, String sig, String[] u) {
                        return new MethodVisitor(Opcodes.ASM9) {
                            @Override
                            public AnnotationVisitor visitAnnotation(String type, boolean synlig) {
                                if (type.equals("Lorg/spongepowered/asm/mixin/Overwrite;")) {
                                    metoder.add(navn + desc);
                                    return null;
                                }
                                return verdier("method", metoder);
                            }
                        };
                    }
                }, ClassReader.SKIP_CODE);
                for (String k : klasser) {
                    Map<String, TreeSet<String>> velgere = ut.computeIfAbsent(k, x -> new TreeMap<>());
                    for (String m : metoder) {
                        velgere.computeIfAbsent(utenEier(m), x -> new TreeSet<>()).add(mod[0]);
                    }
                }
            }
        }
        return ut;
    }

    /** En velger kan ha eieren først ({@code Lzombie/Klasse;navn(…)V}); den er klassen i {@code @Mixin} uansett. */
    static String utenEier(String velger) {
        int parentes = velger.indexOf('(');
        int semikolon = velger.indexOf(';');
        if (velger.startsWith("L") && semikolon > 0 && (parentes < 0 || semikolon < parentes)) {
            return velger.substring(semikolon + 1);
        }
        return velger;
    }

    private static String navnet(String velger) {
        int parentes = velger.indexOf('(');
        return parentes < 0 ? velger : velger.substring(0, parentes);
    }

    /** « (InnloggingMixin)»: Java-modene som griper inn i metoden, til listen. */
    static String hvem(String nøkkel, Map<String, Map<String, TreeSet<String>>> mål) {
        int skille = nøkkel.indexOf('#');
        Map<String, TreeSet<String>> velgere = mål.get(skille < 0 ? nøkkel : nøkkel.substring(0, skille));
        if (velgere == null) {
            return "";
        }
        TreeSet<String> mods = new TreeSet<>();
        String navn = skille < 0 ? null : navnet(nøkkel.substring(skille + 1));
        velgere.forEach((velger, m) -> {
            if (navn == null || navnet(velger).equals(navn) || velger.equals("*")) {
                mods.addAll(m);
            }
        });
        return mods.isEmpty() ? "" : " (" + String.join(", ", mods) + ")";
    }

    /** Står metoden fra filen under en klasse eller et navn som er borte nå? Da sier «borte»-linjen det allerede. */
    private static boolean dekket(String nøkkel, Map<String, String> nå) {
        int skille = nøkkel.indexOf('#');
        if (skille < 0) {
            return false;
        }
        String klasse = nøkkel.substring(0, skille);
        return BORTE.equals(nå.get(klasse)) || BORTE.equals(nå.get(klasse + "#" + navnet(nøkkel.substring(skille + 1))));
    }

    /** Verdiene i en liste i en annotasjon ({@code felt == null}: alle lister – {@code value} og {@code targets}). */
    private static AnnotationVisitor verdier(String felt, List<String> ut) {
        return new AnnotationVisitor(Opcodes.ASM9) {
            @Override
            public AnnotationVisitor visitArray(String navn) {
                if (felt != null && !felt.equals(navn)) {
                    return null;
                }
                return new AnnotationVisitor(Opcodes.ASM9) {
                    @Override
                    public void visit(String n, Object verdi) {
                        if (verdi instanceof Type t) {
                            ut.add(t.getClassName());
                        } else if (verdi instanceof String s) {
                            ut.add(felt == null ? s.replace('/', '.') : s);
                        }
                    }
                };
            }
        };
    }

    /** {@code klasse#navnbeskrivelse -> SHA-1}, eller {@link #BORTE} når klassen eller metoden ikke finnes. */
    static Map<String, String> fingeravtrykk(Path jar, Map<String, Map<String, TreeSet<String>>> mål) throws IOException {
        Map<String, String> ut = new TreeMap<>();
        try (ZipFile zip = new ZipFile(jar.toFile())) {
            for (var e : mål.entrySet()) {
                String klasse = e.getKey();
                ZipEntry post = zip.getEntry(klasse.replace('.', '/') + ".class");
                if (post == null) {
                    ut.put(klasse, BORTE);
                    continue;
                }
                ClassNode node = new ClassNode();
                try (InputStream inn = zip.getInputStream(post)) {
                    new ClassReader(inn).accept(node, ClassReader.SKIP_FRAMES);
                }
                for (String velger : e.getValue().keySet()) {
                    int parentes = velger.indexOf('(');
                    String navn = navnet(velger);
                    String desc = parentes < 0 ? null : velger.substring(parentes);
                    boolean funnet = false;
                    for (MethodNode m : node.methods) {
                        if ((navn.equals("*") || m.name.equals(navn)) && (desc == null || m.desc.equals(desc))) {
                            ut.put(klasse + "#" + m.name + m.desc, sha1(tekst(m)));
                            funnet = true;
                        }
                    }
                    if (!funnet) {
                        ut.put(klasse + "#" + velger, BORTE);
                    }
                }
            }
        }
        return ut;
    }

    /** Instruksjonene i metoden som tekst, uten linjenumre og rammer, med merkene nummerert i rekkefølge. */
    static String tekst(MethodNode m) {
        Map<LabelNode, Integer> merker = new IdentityHashMap<>();
        for (AbstractInsnNode i = m.instructions.getFirst(); i != null; i = i.getNext()) {
            if (i instanceof LabelNode l) {
                merker.put(l, merker.size());
            }
        }
        StringBuilder b = new StringBuilder();
        b.append(m.access).append(' ').append(m.name).append(m.desc).append('\n');
        for (AbstractInsnNode i = m.instructions.getFirst(); i != null; i = i.getNext()) {
            if (i instanceof LineNumberNode || i instanceof FrameNode) {
                continue;
            }
            if (i instanceof LabelNode l) {
                b.append('L').append(merker.get(l)).append(":\n");
                continue;
            }
            b.append(i.getOpcode());
            switch (i) {
                case FieldInsnNode f -> b.append(' ').append(f.owner).append('.').append(f.name).append(' ').append(f.desc);
                case MethodInsnNode f -> b.append(' ').append(f.owner).append('.').append(f.name).append(f.desc);
                case TypeInsnNode t -> b.append(' ').append(t.desc);
                case VarInsnNode v -> b.append(' ').append(v.var);
                case IntInsnNode v -> b.append(' ').append(v.operand);
                case IincInsnNode v -> b.append(' ').append(v.var).append(' ').append(v.incr);
                case LdcInsnNode v -> b.append(' ').append(v.cst.getClass().getSimpleName()).append(':').append(v.cst);
                case JumpInsnNode j -> b.append(" L").append(merker.get(j.label));
                case MultiANewArrayInsnNode a -> b.append(' ').append(a.desc).append(' ').append(a.dims);
                case InvokeDynamicInsnNode d -> b.append(' ').append(d.name).append(d.desc).append(' ').append(d.bsm)
                        .append(' ').append(java.util.Arrays.toString(d.bsmArgs));
                case TableSwitchInsnNode t -> {
                    b.append(' ').append(t.min).append(' ').append(t.max).append(" L").append(merker.get(t.dflt));
                    t.labels.forEach(l -> b.append(" L").append(merker.get(l)));
                }
                case LookupSwitchInsnNode t -> {
                    b.append(" L").append(merker.get(t.dflt));
                    for (int n = 0; n < t.keys.size(); n++) {
                        b.append(' ').append(t.keys.get(n)).append(":L").append(merker.get(t.labels.get(n)));
                    }
                }
                default -> { }
            }
            b.append('\n');
        }
        for (TryCatchBlockNode t : m.tryCatchBlocks) {
            b.append("try L").append(merker.get(t.start)).append(" L").append(merker.get(t.end)).append(" L")
                    .append(merker.get(t.handler)).append(' ').append(t.type).append('\n');
        }
        return b.toString();
    }

    static String sha1(String s) {
        try {
            return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(s.getBytes(StandardCharsets.UTF_8)));
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    static String tekst(Map<String, String> fingeravtrykk) {
        StringBuilder b = new StringBuilder("""
                # Avdriftssjekken (launcher.md 3.9 punkt 3): fingeravtrykket (SHA-1 av koden) til hver metode i spillet
                # Java-modene griper inn i, fra versjonen de er prøvd mot. Endres en av dem i en ny versjon av spillet, stopper
                # bygget til noen har lest metoden og sjekket Java-moden; bygget skriver da ut hva filen skal være.
                """);
        fingeravtrykk.forEach((m, f) -> b.append(m).append(' ').append(f).append('\n'));
        return b.toString();
    }

    static Map<String, String> les(String tekst) {
        Map<String, String> ut = new HashMap<>();
        for (String linje : tekst.split("\\R")) {
            String l = linje.strip();
            if (l.isEmpty() || l.startsWith("#")) {
                continue;
            }
            int mellom = l.lastIndexOf(' ');
            if (mellom > 0) {
                ut.put(l.substring(0, mellom), l.substring(mellom + 1));
            }
        }
        return ut;
    }
}
