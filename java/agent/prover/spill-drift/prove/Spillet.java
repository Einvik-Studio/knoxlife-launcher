package prove;

/**
 * Det etterlignede spillet etter en «oppdatering» som Mixin ikke merker: metodene heter det samme, men «hilsen» gjør
 * noe annet. Avdriftssjekken skal finne den, og bare den.
 */
public class Spillet {
    public static String hilsen() {
        return "vanlig, men på en ny måte";
    }

    public int tall() {
        return 1;
    }

    public static void main(String[] args) {
        System.out.println("hilsen=" + hilsen());
    }
}
