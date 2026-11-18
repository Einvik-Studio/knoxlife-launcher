package zombie.core;

// Et etterlignet GameVersion til prøven av knoxlife.porten.Versjon – bare det spillet bruker i Core.
public final class GameVersion {
    private final String tekst;

    public GameVersion(int major, int minor, String suffix) {
        this.tekst = major + "." + minor + (suffix == null ? "" : suffix);
    }

    @Override
    public String toString() {
        return tekst;
    }
}
