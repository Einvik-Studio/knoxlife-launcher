package prove;

/** Et lite etterlignet spill: Mixin-prøven endrer det mens det lastes. */
public class Spillet {
    public static String hilsen() {
        return "vanlig";
    }

    public int tall() {
        return 1;
    }

    public static void main(String[] args) {
        System.out.println("hilsen=" + hilsen());
        System.out.println("tall=" + new Spillet().tall());
        System.out.println("rapport=" + rapport());
    }

    static String rapport() {
        try {
            return (String) Class.forName("knoxlife.agent.KnoxLife").getMethod("rapport").invoke(null);
        } catch (ReflectiveOperationException e) {
            return "ingen agent";
        }
    }
}
