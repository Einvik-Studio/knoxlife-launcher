package knoxlife.mixin;

import com.llamalad7.mixinextras.injector.ModifyReturnValue;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import prove.Spillet;

/** Prøve-moden: én vanlig Mixin og én fra MixinExtras, så begge er prøvd. */
@Mixin(Spillet.class)
public class SpilletMixin {
    @Inject(method = "hilsen", at = @At("HEAD"), cancellable = true)
    private static void knoxlife$hilsen(CallbackInfoReturnable<String> svar) {
        svar.setReturnValue("endret av knoxlife");
    }

    @ModifyReturnValue(method = "tall", at = @At("RETURN"))
    private int knoxlife$tall(int tall) {
        return tall + 41;
    }
}
