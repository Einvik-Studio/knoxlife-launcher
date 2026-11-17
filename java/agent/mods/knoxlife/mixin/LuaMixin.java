package knoxlife.mixin;

import knoxlife.agent.KnoxLifeJava;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import zombie.Lua.LuaManager;

/**
 * Gjør {@link KnoxLifeJava} synlig for Lua som den globale {@code KnoxLifeJava} (launcher.md del 4), slik spillet gjør
 * med sine egne klasser: den legges i listen før spillet går gjennom den. Modet spør gjennom {@code KnoxLife.Java}.
 */
@Mixin(LuaManager.Exposer.class)
public abstract class LuaMixin {
    @Shadow
    public abstract void setExposed(Class<?> klasse);

    @Inject(method = "exposeAll", at = @At("HEAD"))
    private void knoxlife$vis(CallbackInfo ci) {
        setExposed(KnoxLifeJava.class);
    }
}
