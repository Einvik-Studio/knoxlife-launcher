package knoxlife.agent.mixin;

import com.llamalad7.mixinextras.MixinExtrasBootstrap;
import java.lang.instrument.Instrumentation;
import java.lang.reflect.Method;
import org.spongepowered.asm.launch.MixinBootstrap;
import org.spongepowered.asm.mixin.MixinEnvironment;
import org.spongepowered.asm.mixin.Mixins;

/**
 * Starter Mixin med Java-modene våre ({@code knoxlife.mixins.json}, og på serveren {@code knoxlife-server.mixins.json})
 * og kobler det til Java før spillets klasser lastes.
 */
public final class Start {
    public static final String OPPSETT = "knoxlife.mixins.json";
    /** Serverdelens Java-mods – finnes bare i serverens agent, som bygges i det private repoet. */
    public static final String SERVERENS_OPPSETT = "knoxlife-server.mixins.json";

    private Start() {}

    public static void start(Instrumentation inst) throws ReflectiveOperationException {
        ClassLoader laster = ClassLoader.getSystemClassLoader();
        Tjeneste.laster = laster;
        MixinBootstrap.init();
        Mixins.addConfiguration(OPPSETT);
        if (laster.getResource(SERVERENS_OPPSETT) != null) {
            Mixins.addConfiguration(SERVERENS_OPPSETT);
        }
        // Omformeren lages før MixinExtras, som henger sine utvidelser på den.
        var omformer = Tjeneste.omformer();
        MixinExtrasBootstrap.init();
        // Mixin går gjennom fasene selv i en modlaster. Her er spillet klart med en gang, så vi går rett til DEFAULT.
        Method fase = MixinEnvironment.class.getDeclaredMethod("gotoPhase", MixinEnvironment.Phase.class);
        fase.setAccessible(true);
        fase.invoke(null, MixinEnvironment.Phase.INIT);
        fase.invoke(null, MixinEnvironment.Phase.DEFAULT);
        inst.addTransformer(new Omformer(omformer, laster), false);
    }
}
