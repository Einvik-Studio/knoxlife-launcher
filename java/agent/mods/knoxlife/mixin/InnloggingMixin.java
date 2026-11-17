package knoxlife.mixin;

import knoxlife.agent.KnoxLife;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import zombie.core.Core;
import zombie.core.network.ByteBufferWriter;
import zombie.network.packets.connection.LoginPacket;

/**
 * J1, juksesperren hos spilleren (launcher.md 3.3 punkt 1): rapporten legges bakerst i innloggingen, etter feltene
 * spillet selv sender. En server uten Knox Life leser den ikke; serveren vår leser den før noe annet avgjøres.
 */
@Mixin(LoginPacket.class)
public abstract class InnloggingMixin {
    @Inject(method = "write", at = @At("TAIL"))
    private void knoxlife$rapport(ByteBufferWriter b, CallbackInfo ci) {
        b.putUTF(KnoxLife.rapport(Core.debug));
    }
}
