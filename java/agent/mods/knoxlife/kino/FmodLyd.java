package knoxlife.kino;

import fmod.javafmod;
import java.nio.file.Path;
import knoxlife.agent.Kino;

/**
 * Lydsporet til filmen gjennom spillets egen lydmotor (FMOD, J11, kino.md 3.9 punkt 5): strømmet fra filen, i to
 * kanaler (2D), uten løkke og med nøyaktig tid, så den kan hoppe dit filmen er. Volumet regner Lua ut: bilradioen på
 * kinoens frekvens, eller nærheten til en høyttalerstolpe.
 */
public final class FmodLyd implements Kino.Lyd {
    private static final long MODUS = 128 | 8 | 1 | 16384; // FMOD_CREATESTREAM | FMOD_2D | FMOD_LOOP_OFF | FMOD_ACCURATETIME
    private long lyd;
    private long kanal;
    private float volum = 0f;

    @Override
    public void spill(Path ogg, long ms) {
        stopp();
        lyd = javafmod.FMOD_System_CreateSound(ogg.toString(), MODUS);
        if (lyd == 0) {
            return;
        }
        kanal = javafmod.FMOD_System_PlaySound(lyd, true);
        if (kanal == 0) {
            return;
        }
        javafmod.FMOD_Channel_SetPosition(kanal, Math.max(0, ms));
        javafmod.FMOD_Channel_SetVolume(kanal, volum);
        javafmod.FMOD_Channel_SetPaused(kanal, false);
    }

    @Override
    public void pause(boolean pause) {
        if (kanal != 0) {
            javafmod.FMOD_Channel_SetPaused(kanal, pause);
        }
    }

    @Override
    public void volum(float v) {
        volum = Math.max(0f, Math.min(1f, v));
        if (kanal != 0) {
            javafmod.FMOD_Channel_SetVolume(kanal, volum);
        }
    }

    @Override
    public void stopp() {
        if (kanal != 0) {
            javafmod.FMOD_Channel_Stop(kanal);
            kanal = 0;
        }
        if (lyd != 0) {
            javafmod.FMOD_Sound_Release(lyd);
            lyd = 0;
        }
    }
}
