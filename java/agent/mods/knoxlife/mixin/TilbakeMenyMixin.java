package knoxlife.mixin;

import knoxlife.agent.Profil;
import knoxlife.tilbake.Spillet;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import zombie.gameStates.GameStateMachine;
import zombie.gameStates.MainScreenState;

/**
 * J6, hovedmenyen (launcher.md 3.6 punkt 7): etter et brudd kobler spillet seg til Knox Life igjen gjennom spillets egen
 * skjerm for tilkobling, hvert 15. sekund i opptil 15 minutter. Avgjørelsene er i {@code knoxlife.agent.Tilbake}.
 *
 * <p>J7 og J9 (3.8 punkt 1 og 4): i menyen er spilleren ute av Knox Life – listen over hendelser som hoppes over, og
 * profileren nullstilles ({@link Profil#iMenyen()}), så en annen server får spillet som det er.
 */
@Mixin(MainScreenState.class)
public abstract class TilbakeMenyMixin {
    @Inject(method = "update", at = @At("HEAD"))
    private void knoxlife$iMenyen(CallbackInfoReturnable<GameStateMachine.StateAction> cir) {
        Profil.SPILLET.iMenyen();
        Spillet.iMenyen();
    }
}
