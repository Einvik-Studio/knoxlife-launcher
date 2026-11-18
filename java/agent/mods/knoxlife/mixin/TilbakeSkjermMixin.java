package knoxlife.mixin;

import knoxlife.tilbake.Spillet;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import zombie.gameStates.GameStateMachine;
import zombie.gameStates.ServerDisconnectState;

/**
 * J6, skjermen ved bruddet (launcher.md 3.6 punkt 7): faller forbindelsen til Knox Life uten at spilleren er kastet
 * ut, sier skjermen at spilleren kommer inn igjen av seg selv, og etter 3 sekunder går spillet til hovedmenyen.
 * Avgjørelsene er i {@code knoxlife.agent.Tilbake}.
 */
@Mixin(ServerDisconnectState.class)
public abstract class TilbakeSkjermMixin {
    @Inject(method = "enter", at = @At("HEAD"))
    private void knoxlife$brudd(CallbackInfo ci) {
        Spillet.vedBrudd();
    }

    @Inject(method = "update", at = @At("HEAD"))
    private void knoxlife$tilMenyen(CallbackInfoReturnable<GameStateMachine.StateAction> cir) {
        Spillet.påSkjermen();
    }
}
