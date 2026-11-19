package knoxlife.kino;

import java.nio.ByteBuffer;
import knoxlife.agent.Kino;
import knoxlife.agent.KnoxLife;
import zombie.core.SpriteRenderer;
import zombie.core.textures.Texture;
import zombie.core.textures.TextureDraw;
import zombie.iso.IsoCamera;
import zombie.iso.IsoUtils;

/**
 * Tegner filmen på lerretet (J11). Bildet lastes opp til én tekstur i spillets egen tegnetråd ({@code drawGeneric}), og
 * tegnes som en firkant mellom lerretets nedre kant og kanten {@code etasjer} opp, i spillets vinkel
 * ({@code IsoUtils.XToScreen} minus kameraets forskyvning, slik spillet selv regner for flisene). Bare når kameraet er
 * innen {@link #SYNSVIDDE} ruter fra lerretet.
 */
public final class Lerret {
    /** Ruter fra lerretet der det tegnes. */
    static final double SYNSVIDDE = 90;
    private static volatile Texture tekstur;
    private static long lastet = -1;
    private static boolean feilet;

    private Lerret() {}

    public static void tegn(int spiller) {
        if (feilet) {
            return;
        }
        try {
            Kino k = Kino.SPILLET;
            double[] l = k.lerret();
            if (l == null || !k.aktiv()) {
                return;
            }
            IsoCamera.FrameState f = IsoCamera.frameState;
            double mx = (l[0] + l[2]) / 2;
            double my = (l[1] + l[3]) / 2;
            if (f.camCharacter == null || Math.hypot(f.camCharacterX - mx, f.camCharacterY - my) > SYNSVIDDE
                    || Math.abs(f.camCharacterZ - l[4]) > 2) {
                return;
            }
            Kino.Ramme r = k.tegnes();
            if (r == null) {
                return;
            }
            if (r.nr() != lastet) {
                lastet = r.nr();
                SpriteRenderer.instance.drawGeneric(new Opplasting(r.data()));
            }
            Texture t = tekstur;
            if (t == null) {
                return; // teksturen lages i tegnetråden; neste bilde
            }
            float z0 = (float) l[4];
            float z1 = (float) (l[4] + l[5]);
            float ox = f.offX;
            float oy = f.offY;
            float x1 = (float) l[0], y1 = (float) l[1], x2 = (float) l[2], y2 = (float) l[3];
            float a = (float) (0.3 + 0.7 * k.lys());
            // Øverst til venstre, øverst til høyre, nederst til høyre, nederst til venstre.
            SpriteRenderer.instance.render(t,
                    IsoUtils.XToScreen(x1, y1, z1, 0) - ox, IsoUtils.YToScreen(x1, y1, z1, 0) - oy,
                    IsoUtils.XToScreen(x2, y2, z1, 0) - ox, IsoUtils.YToScreen(x2, y2, z1, 0) - oy,
                    IsoUtils.XToScreen(x2, y2, z0, 0) - ox, IsoUtils.YToScreen(x2, y2, z0, 0) - oy,
                    IsoUtils.XToScreen(x1, y1, z0, 0) - ox, IsoUtils.YToScreen(x1, y1, z0, 0) - oy,
                    1f, 1f, 1f, a, null);
        } catch (Throwable e) {
            feilet = true;
            KnoxLife.feil("kino: lerretet", e);
        }
    }

    /** Laster opp bildet i spillets tegnetråd; lager teksturen første gang. */
    static final class Opplasting extends TextureDraw.GenericDrawer {
        private final ByteBuffer data;

        Opplasting(ByteBuffer data) {
            this.data = data;
        }

        @Override
        public void render() {
            try {
                Texture t = tekstur;
                if (t == null) {
                    t = new Texture(1024, 512, 0);
                    tekstur = t;
                }
                data.position(0);
                t.setData(data);
            } catch (Throwable e) {
                feilet = true;
                KnoxLife.feil("kino: teksturen", e);
            }
        }
    }
}
