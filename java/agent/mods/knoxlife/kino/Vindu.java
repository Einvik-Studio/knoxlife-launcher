package knoxlife.kino;

import knoxlife.agent.Kino;
import zombie.core.SpriteRenderer;
import zombie.core.textures.Texture;

/**
 * Vinduet «Watch» (vedtatt 19.11.2026: «Lerretet og et vindu»): filmen stort i et vindu i spillet, tegnet fra vinduets
 * {@code render()} i UI-laget. Sort bak (uten tekstur gir spillet en fylt firkant, som Lua sin {@code drawRect}), så
 * bildet i 4:3 i midten med samme tekstur som lerretet – alltid klart, uten dagslyset.
 */
public final class Vindu implements Kino.Vindu {
    @Override
    public void tegn(Kino.Ramme bilde, double x, double y, double b, double h, double[] f) {
        if (Lerret.feilet()) {
            return;
        }
        try {
            SpriteRenderer.instance.renderi(null, (int) x, (int) y, (int) Math.ceil(b), (int) Math.ceil(h), 0f, 0f, 0f, 1f,
                    null);
            if (bilde == null) {
                return;
            }
            Texture t = Lerret.tekstur(bilde);
            if (t == null) {
                return;
            }
            double x1 = f[0] + f[2];
            double y1 = f[1] + f[3];
            SpriteRenderer.instance.render(t, f[0], f[1], x1, f[1], x1, y1, f[0], y1, 1f, 1f, 1f, 1f, null);
        } catch (Throwable e) {
            Lerret.feil("kino: vinduet", e);
        }
    }
}
