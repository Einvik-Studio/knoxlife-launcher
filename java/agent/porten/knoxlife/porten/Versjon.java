package knoxlife.porten;

import java.io.IOException;
import java.io.InputStream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.FieldInsnNode;
import org.objectweb.asm.tree.FieldNode;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.IntInsnNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;

/**
 * Spillets versjon slik spillet selv viser den («42.22.2»), lest ut av {@code zombie/core/Core.class} uten å starte
 * spillet – til manifestet i utgivelsen (launcher.md 3.10). Spillet setter den sammen av {@code gameVersion} (en
 * {@code GameVersion} med hovednummer, undernummer og et tillegg, laget i klassens statiske del) og konstanten
 * {@code buildVersion}: {@code getGameAndBuildVersion()} gir {@code gameVersion + "." + buildVersion}.
 *
 * <pre>java -cp &lt;asm&gt;:&lt;porten&gt; knoxlife.porten.Versjon &lt;projectzomboid.jar&gt;</pre>
 *
 * Finner den ikke begge delene, stopper den med en feil – da har spillet endret måten versjonen lages på, og verktøyet må
 * rettes før en utgivelse.
 */
public final class Versjon {
    static final String CORE = "zombie/core/Core";
    static final String GAME_VERSION = "zombie/core/GameVersion";

    private Versjon() {}

    public static void main(String[] args) throws IOException {
        if (args.length != 1) {
            System.err.println("bruk: Versjon <projectzomboid.jar>");
            System.exit(2);
        }
        try (ZipFile jar = new ZipFile(args[0])) {
            ZipEntry e = jar.getEntry(CORE + ".class");
            if (e == null) {
                feil("fant ikke " + CORE + ".class");
            }
            try (InputStream inn = jar.getInputStream(e)) {
                System.out.println(les(inn.readAllBytes()));
            }
        } catch (IllegalStateException ex) {
            feil(ex.getMessage());
        }
    }

    private static void feil(String melding) {
        System.err.println("Versjon: " + melding);
        System.exit(1);
    }

    /** Versjonen fra bytene i Core.class, eller {@link IllegalStateException} når den ikke kan leses sikkert. */
    static String les(byte[] klasse) {
        ClassNode c = new ClassNode();
        new ClassReader(klasse).accept(c, 0);
        Integer bygg = null;
        for (FieldNode f : c.fields) {
            if (f.name.equals("buildVersion") && f.desc.equals("I") && (f.access & Opcodes.ACC_STATIC) != 0
                    && f.value instanceof Integer i) {
                bygg = i;
            }
        }
        if (bygg == null) {
            throw new IllegalStateException("fant ikke konstanten buildVersion i " + CORE);
        }
        for (MethodNode m : c.methods) {
            if (!m.name.equals("<clinit>")) {
                continue;
            }
            for (AbstractInsnNode i = m.instructions.getFirst(); i != null; i = i.getNext()) {
                if (i instanceof FieldInsnNode f && f.getOpcode() == Opcodes.PUTSTATIC && f.owner.equals(c.name)
                        && f.name.equals("gameVersion")) {
                    return spillversjon(f) + "." + bygg;
                }
            }
        }
        throw new IllegalStateException("fant ikke gameVersion i den statiske delen av " + CORE);
    }

    /** {@code new GameVersion(major, minor, suffix)} rett før feltet settes – ellers en feil. */
    private static String spillversjon(FieldInsnNode put) {
        AbstractInsnNode init = forrige(put);
        if (!(init instanceof MethodInsnNode m) || m.getOpcode() != Opcodes.INVOKESPECIAL || !m.owner.equals(GAME_VERSION)
                || !m.name.equals("<init>") || !m.desc.equals("(IILjava/lang/String;)V")) {
            throw new IllegalStateException("gameVersion lages ikke med new GameVersion(int, int, String)");
        }
        AbstractInsnNode tillegg = forrige(init);
        AbstractInsnNode under = forrige(tillegg);
        AbstractInsnNode hoved = forrige(under);
        String s;
        if (tillegg instanceof LdcInsnNode l && l.cst instanceof String t) {
            s = t;
        } else if (tillegg instanceof InsnNode n && n.getOpcode() == Opcodes.ACONST_NULL) {
            s = "";
        } else {
            throw new IllegalStateException("tillegget i GameVersion er ikke en konstant");
        }
        return tall(hoved) + "." + tall(under) + s;
    }

    private static AbstractInsnNode forrige(AbstractInsnNode i) {
        AbstractInsnNode p = i == null ? null : i.getPrevious();
        while (p != null && p.getOpcode() < 0) { // merker, linjenumre og rammer
            p = p.getPrevious();
        }
        return p;
    }

    private static int tall(AbstractInsnNode i) {
        if (i == null) {
            throw new IllegalStateException("versjonen i GameVersion er ikke konstanter");
        }
        int op = i.getOpcode();
        if (op >= Opcodes.ICONST_M1 && op <= Opcodes.ICONST_5) {
            return op - Opcodes.ICONST_0;
        }
        if (i instanceof IntInsnNode n && (op == Opcodes.BIPUSH || op == Opcodes.SIPUSH)) {
            return n.operand;
        }
        if (i instanceof LdcInsnNode l && l.cst instanceof Integer v) {
            return v;
        }
        throw new IllegalStateException("versjonen i GameVersion er ikke konstanter");
    }
}
