package knoxlife.agent.mixin;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;
import knoxlife.agent.KnoxLife;
import org.spongepowered.asm.mixin.transformer.IMixinTransformer;

/** Gir hver av spillets klasser til Mixin idet den lastes, så Java-modene settes inn før klassen brukes. */
final class Omformer implements ClassFileTransformer {
    private static final String[] HOPP_OVER = {
        "java/", "javax/", "jdk/", "sun/", "com/sun/",
        "org/spongepowered/", "org/objectweb/", "com/llamalad7/", "knoxlife/agent/",
    };

    private final IMixinTransformer mixin;
    private final ClassLoader laster;

    Omformer(IMixinTransformer mixin, ClassLoader laster) {
        this.mixin = mixin;
        this.laster = laster;
    }

    static boolean hoppOver(String navn) {
        for (String p : HOPP_OVER) {
            if (navn.startsWith(p)) {
                return true;
            }
        }
        return false;
    }

    @Override
    public byte[] transform(ClassLoader l, String navn, Class<?> omdefinert, ProtectionDomain pd, byte[] bytes) {
        if (navn == null || omdefinert != null || l != laster || hoppOver(navn)) {
            return null;
        }
        String punktum = navn.replace('/', '.');
        try {
            byte[] ut = mixin.transformClassBytes(punktum, punktum, bytes);
            return ut == bytes ? null : ut;
        } catch (Throwable t) {
            // Java svelger feil fra en omformer og laster klassen uendret. Feilen skrives og sendes i rapporten, så
            // serveren vet at patchen ikke er hel.
            KnoxLife.feil(punktum, t);
            return null;
        }
    }
}
