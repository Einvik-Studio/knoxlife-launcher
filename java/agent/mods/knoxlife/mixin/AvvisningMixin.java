package knoxlife.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import zombie.Lua.LuaEventManager;
import zombie.core.raknet.UdpConnection;
import zombie.network.packets.service.AccessDeniedPacket;

/**
 * Viser serverens egen tekst når Knox Life avviser innloggingen (launcher.md 3.3 punkt 2). Spillet slår ellers opp
 * grunnen i sine egne tekster ({@code UI_OnConnectFailed_<nøkkel>}), og der finnes ingen tekst for oss. Grunnen
 * {@code KnoxLife##<tekst>} vises som den er; alle andre grunner viser spillet som før.
 */
@Mixin(AccessDeniedPacket.class)
public abstract class AvvisningMixin {
    private static final String MERKE = "KnoxLife##";

    @Shadow
    String reason;

    @Inject(method = "processClientLoading", at = @At("HEAD"), cancellable = true)
    private void knoxlife$vis(UdpConnection forbindelse, CallbackInfo ci) {
        if (reason != null && reason.startsWith(MERKE)) {
            LuaEventManager.triggerEvent("OnConnectFailed", reason.substring(MERKE.length()));
            ci.cancel();
        }
    }
}
