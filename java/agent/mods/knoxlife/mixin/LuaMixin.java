package knoxlife.mixin;

import knoxlife.agent.Kino;
import knoxlife.agent.KnoxKino;
import knoxlife.agent.KnoxLifeJava;
import knoxlife.kino.FmodLyd;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import zombie.Lua.LuaManager;

/**
 * Gjør {@link KnoxLifeJava} synlig for Lua som den globale {@code KnoxLifeJava} (launcher.md del 4), og kinoen som
 * {@link KnoxKino} (J11), slik spillet gjør med sine egne klasser: de legges i listen før spillet går gjennom den.
 * Modet spør gjennom {@code KnoxLife.Java} og {@code KnoxLife.Kino}.
 */
@Mixin(LuaManager.Exposer.class)
public abstract class LuaMixin {
    @Shadow
    public abstract void setExposed(Class<?> klasse);

    @Inject(method = "exposeAll", at = @At("HEAD"))
    private void knoxlife$vis(CallbackInfo ci) {
        setExposed(KnoxLifeJava.class);
        // J11: kinoen (kino.md 3.9) – den globale KnoxKino, og lyden gjennom spillets FMOD. På serveren tegnes og
        // spilles ingenting: Lua kaller den bare hos spilleren.
        setExposed(KnoxKino.class);
        Kino.SPILLET.settLyd(new FmodLyd());
    }
}
