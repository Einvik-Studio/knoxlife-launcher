package knoxlife.profil;

import knoxlife.agent.Profil;
import se.krka.kahlua.vm.Prototype;

/**
 * Hvilken fil og linje en Lua-funksjon er, slik Kahlua har dem ({@code Prototype}). Bygget mot spillet, så klassen lastes
 * først når {@link Profil} trenger den, og melder seg der.
 */
public final class Navn {
    static {
        Profil.SPILLET.navngi(Navn::funksjon);
    }

    private Navn() {}

    static Profil.Funksjon funksjon(Object nøkkel) {
        if (!(nøkkel instanceof Prototype p)) {
            return Profil.Funksjon.UKJENT;
        }
        String sti = p.file != null && !p.file.isBlank() ? p.file : p.filename != null ? p.filename : p.name;
        int linje = p.lines != null && p.lines.length > 0 ? p.lines[0] : 0;
        return Profil.Funksjon.av(sti, linje);
    }
}
