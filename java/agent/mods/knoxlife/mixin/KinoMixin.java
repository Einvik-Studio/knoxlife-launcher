package knoxlife.mixin;

import knoxlife.kino.Lerret;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import zombie.iso.IsoCell;
import zombie.iso.fboRenderChunk.FBORenderCell;

/**
 * J11 kinoen (kino.md 3.9 punkt 2): lerretet tegnes i hvert bilde, rett før spillet tegner det som beveger seg (folk og
 * biler). Da ligger filmen over flisene til lerretet, og en bil eller en person foran lerretet står foran bildet. Bygningene
 * og flisene er alt tegnet fra bitene spillet har i minnet ({@code FBORenderChunk}), så filmen må tegnes her og ikke
 * sammen med flisene – de tegnes ikke i hvert bilde.
 */
@Mixin(FBORenderCell.class)
public abstract class KinoMixin {
    @Inject(method = "performRenderTiles", at = @At(value = "INVOKE",
            target = "Lzombie/iso/fboRenderChunk/FBORenderCell;renderMovingObjects()V"))
    private void knoxlife$lerret(IsoCell.PerPlayerRender ppr, int spiller, long tid, CallbackInfo ci) {
        Lerret.tegn(spiller);
    }
}
