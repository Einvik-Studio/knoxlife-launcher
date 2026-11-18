package zombie.core;

// Et etterlignet Core til prøven av knoxlife.porten.Versjon: versjonen lages slik spillet lager sin (B42.22), med et
// undernummer over 127, så det står som SIPUSH og ikke BIPUSH i klassefilen.
public final class Core {
    private static final GameVersion gameVersion = new GameVersion(42, 130, "");
    private static final int buildVersion = 2;
    private static String gitRevision = "prove";

    public String getGameAndBuildVersion() {
        return gameVersion + "." + buildVersion;
    }
}
