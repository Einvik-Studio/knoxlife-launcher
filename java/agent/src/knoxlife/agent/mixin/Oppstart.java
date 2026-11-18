package knoxlife.agent.mixin;

import org.spongepowered.asm.service.IMixinServiceBootstrap;

/** Sier til Mixin hvilken tjeneste som gjelder. */
public final class Oppstart implements IMixinServiceBootstrap {
    @Override
    public String getName() {
        return "KnoxLife";
    }

    @Override
    public String getServiceClassName() {
        return Tjeneste.class.getName();
    }

    @Override
    public void bootstrap() {
        // Ingenting å gjøre: klassene ligger alt i spillets klasselaster.
    }
}
