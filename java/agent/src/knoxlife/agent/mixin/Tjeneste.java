package knoxlife.agent.mixin;

import java.io.IOException;
import java.io.InputStream;
import java.net.URL;
import java.util.Collection;
import java.util.List;
import knoxlife.agent.Agent;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.tree.ClassNode;
import org.spongepowered.asm.launch.platform.container.ContainerHandleVirtual;
import org.spongepowered.asm.launch.platform.container.IContainerHandle;
import org.spongepowered.asm.mixin.MixinEnvironment.CompatibilityLevel;
import org.spongepowered.asm.mixin.transformer.IMixinTransformer;
import org.spongepowered.asm.mixin.transformer.IMixinTransformerFactory;
import org.spongepowered.asm.service.IAdviceProvider;
import org.spongepowered.asm.service.IClassBytecodeProvider;
import org.spongepowered.asm.service.IClassProvider;
import org.spongepowered.asm.service.IClassTracker;
import org.spongepowered.asm.service.IFeatureValidator;
import org.spongepowered.asm.service.IMixinAuditTrail;
import org.spongepowered.asm.service.IMixinInternal;
import org.spongepowered.asm.service.ITransformerProvider;
import org.spongepowered.asm.service.MixinServiceAbstract;

/**
 * Mixin trenger en «tjeneste» som sier hvor klassene finnes og hvordan de leses. Spillet har ingen modlaster, så vi
 * har vår egen: klassene ligger i spillets egen klasselaster (systemlasteren), og bytene leses derfra.
 */
public final class Tjeneste extends MixinServiceAbstract implements IClassProvider, IClassBytecodeProvider {
    static volatile ClassLoader laster = ClassLoader.getSystemClassLoader();
    private static volatile IMixinTransformerFactory fabrikk;
    private static IMixinTransformer omformer;

    @Override
    public String getName() {
        return "KnoxLife";
    }

    @Override
    public boolean isValid() {
        return Agent.startet();
    }

    @Override
    public void offer(IMixinInternal internal) {
        if (internal instanceof IMixinTransformerFactory f) {
            fabrikk = f;
        }
        super.offer(internal);
    }

    static synchronized IMixinTransformer omformer() {
        if (omformer == null) {
            if (fabrikk == null) {
                throw new IllegalStateException("Mixin did not offer a transformer");
            }
            omformer = fabrikk.createTransformer();
        }
        return omformer;
    }

    @Override
    public CompatibilityLevel getMaxCompatibilityLevel() {
        return CompatibilityLevel.JAVA_25;
    }

    @Override
    public IClassProvider getClassProvider() {
        return this;
    }

    @Override
    public IClassBytecodeProvider getBytecodeProvider() {
        return this;
    }

    @Override
    public ITransformerProvider getTransformerProvider() {
        return null;
    }

    @Override
    public IClassTracker getClassTracker() {
        return null;
    }

    @Override
    public IMixinAuditTrail getAuditTrail() {
        return null;
    }

    @Override
    public IAdviceProvider getAdviceProvider() {
        return null;
    }

    @Override
    public IFeatureValidator getFeatureValidator() {
        return null;
    }

    @Override
    public Collection<String> getPlatformAgents() {
        return List.of("org.spongepowered.asm.launch.platform.MixinPlatformAgentDefault");
    }

    @Override
    public IContainerHandle getPrimaryContainer() {
        return new ContainerHandleVirtual(getName());
    }

    @Override
    public InputStream getResourceAsStream(String navn) {
        return laster.getResourceAsStream(navn);
    }

    // ---------- IClassProvider ----------

    @Override
    @Deprecated
    public URL[] getClassPath() {
        return new URL[0];
    }

    @Override
    public Class<?> findClass(String navn) throws ClassNotFoundException {
        return Class.forName(navn, true, laster);
    }

    @Override
    public Class<?> findClass(String navn, boolean initier) throws ClassNotFoundException {
        return Class.forName(navn, initier, laster);
    }

    @Override
    public Class<?> findAgentClass(String navn, boolean initier) throws ClassNotFoundException {
        return Class.forName(navn, initier, Tjeneste.class.getClassLoader());
    }

    // ---------- IClassBytecodeProvider ----------

    @Override
    public ClassNode getClassNode(String navn) throws ClassNotFoundException, IOException {
        return getClassNode(navn, true, 0);
    }

    @Override
    public ClassNode getClassNode(String navn, boolean kjørOmformere) throws ClassNotFoundException, IOException {
        return getClassNode(navn, kjørOmformere, 0);
    }

    @Override
    public ClassNode getClassNode(String navn, boolean kjørOmformere, int flagg) throws ClassNotFoundException, IOException {
        String fil = navn.replace('.', '/') + ".class";
        try (InputStream inn = laster.getResourceAsStream(fil)) {
            if (inn == null) {
                throw new ClassNotFoundException(navn);
            }
            ClassNode node = new ClassNode();
            new ClassReader(inn.readAllBytes()).accept(node, flagg);
            return node;
        }
    }
}
