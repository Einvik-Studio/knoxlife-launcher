package knoxlife.mixin;

import knoxlife.agent.Profil;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import se.krka.kahlua.integration.LuaCaller;
import se.krka.kahlua.vm.KahluaTable;
import se.krka.kahlua.vm.KahluaThread;
import se.krka.kahlua.vm.LuaClosure;
import zombie.Lua.Event;

/**
 * J7 profileren og J9 hendelsene som hoppes over (launcher.md 3.8 punkt 1 og 4). Spillets {@code Event.trigger} kaller
 * hver Lua-funksjon som lytter på hendelsen med {@code LuaCaller.protectedCallVoid}; her går hvert slikt kall gjennom
 * {@link Profil}, som tar tiden – eller hopper over kallet når funksjonen står i listen. Tikket måles fra hendelsen
 * {@link Profil#TIKK} til den neste. Hvilken fil og linje en funksjon er, sier {@code knoxlife.profil.Navn}.
 */
@Mixin(Event.class)
public abstract class ProfilMixin {
    @Shadow
    public String name;

    @Inject(method = "trigger", at = @At("HEAD"))
    private void knoxlife$hendelse(KahluaTable env, LuaCaller caller, Object[] params,
            CallbackInfoReturnable<Boolean> cir) {
        Profil.SPILLET.hendelse(name, System.nanoTime());
    }

    @Redirect(method = "trigger", at = @At(value = "INVOKE", target = "Lse/krka/kahlua/integration/LuaCaller;"
            + "protectedCallVoid(Lse/krka/kahlua/vm/KahluaThread;Ljava/lang/Object;[Ljava/lang/Object;)V"))
    private void knoxlife$kall(LuaCaller caller, KahluaThread tråd, Object funksjon, Object[] argumenter) {
        Object nøkkel = funksjon instanceof LuaClosure c ? c.prototype : funksjon;
        Object teller = Profil.SPILLET.før(this, name, nøkkel);
        if (teller == null) {
            return; // J9: står i listen over hendelser som hoppes over
        }
        long start = System.nanoTime();
        try {
            caller.protectedCallVoid(tråd, funksjon, argumenter);
        } finally {
            Profil.SPILLET.etter(teller, System.nanoTime() - start);
        }
    }
}
