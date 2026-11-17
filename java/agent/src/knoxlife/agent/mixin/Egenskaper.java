package knoxlife.agent.mixin;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import org.spongepowered.asm.service.IGlobalPropertyService;
import org.spongepowered.asm.service.IPropertyKey;

/** Mixins egne verdier mens spillet går – i minnet, ingen andre steder. */
public final class Egenskaper implements IGlobalPropertyService {
    private record Navn(String navn) implements IPropertyKey {}

    private final Map<String, Object> verdier = new ConcurrentHashMap<>();

    @Override
    public IPropertyKey resolveKey(String navn) {
        return new Navn(navn);
    }

    @Override
    @SuppressWarnings("unchecked")
    public <T> T getProperty(IPropertyKey nøkkel) {
        return (T) verdier.get(((Navn) nøkkel).navn());
    }

    @Override
    public void setProperty(IPropertyKey nøkkel, Object verdi) {
        if (verdi == null) {
            verdier.remove(((Navn) nøkkel).navn());
        } else {
            verdier.put(((Navn) nøkkel).navn(), verdi);
        }
    }

    @Override
    @SuppressWarnings("unchecked")
    public <T> T getProperty(IPropertyKey nøkkel, T standard) {
        Object v = verdier.get(((Navn) nøkkel).navn());
        return v == null ? standard : (T) v;
    }

    @Override
    public String getPropertyString(IPropertyKey nøkkel, String standard) {
        Object v = verdier.get(((Navn) nøkkel).navn());
        return v == null ? standard : v.toString();
    }
}
