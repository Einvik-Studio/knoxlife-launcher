package knoxlife.agent;

import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import javax.imageio.ImageIO;

/**
 * Bildene til lerretet (J11): JPEG fra filmen, eller en tekst («Intermission – 10:00»), lagt inn i et RGBA-område på
 * {@link #BREDDE} × {@link #HOYDE} – teksturen i spillet er så stor (to-potens), og filmen fyller {@link #FILM_BREDDE} ×
 * {@link #FILM_HOYDE} øverst til venstre. Uskarpt fokus er en enkel boksuskarphet. Spillets egen Java har
 * {@code java.desktop}, så ImageIO og AWT virker (også uten skjerm, {@code java.awt.headless}).
 */
final class KinoBilde {
    static final int BREDDE = 1024;
    static final int HOYDE = 512;
    static final int FILM_BREDDE = 640;
    static final int FILM_HOYDE = 480;
    /** Mest uskarphet i punkter når fokus er 0. */
    static final int USKARP_HOYST = 6;

    private KinoBilde() {}

    static ByteBuffer nyBuffer() {
        return ByteBuffer.allocateDirect(BREDDE * HOYDE * 4);
    }

    /** Fokus 0..1 til radius i punkter. */
    static int radius(double fokus) {
        double f = Math.max(0, Math.min(1, fokus));
        return (int) Math.round((1 - f) * USKARP_HOYST);
    }

    static void jpeg(byte[] jpeg, int radius, ByteBuffer ut) throws IOException {
        BufferedImage b = ImageIO.read(new ByteArrayInputStream(jpeg));
        if (b == null) {
            throw new IOException("not a JPEG");
        }
        legg(b, radius, ut);
    }

    static void tekst(String linje, ByteBuffer ut) {
        BufferedImage b = new BufferedImage(FILM_BREDDE, FILM_HOYDE, BufferedImage.TYPE_INT_RGB);
        Graphics2D g = b.createGraphics();
        try {
            g.setColor(new Color(18, 18, 20));
            g.fillRect(0, 0, FILM_BREDDE, FILM_HOYDE);
            g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
            int størrelse = 44;
            Font f = new Font(Font.SANS_SERIF, Font.BOLD, størrelse);
            g.setFont(f);
            FontMetrics m = g.getFontMetrics();
            while (m.stringWidth(linje) > FILM_BREDDE - 40 && størrelse > 16) {
                størrelse -= 2;
                g.setFont(new Font(Font.SANS_SERIF, Font.BOLD, størrelse));
                m = g.getFontMetrics();
            }
            g.setColor(new Color(240, 199, 94));
            g.drawString(linje, (FILM_BREDDE - m.stringWidth(linje)) / 2, (FILM_HOYDE + m.getAscent() - m.getDescent()) / 2);
        } finally {
            g.dispose();
        }
        legg(b, 0, ut);
    }

    /** Bildet skaleres til filmflaten og legges i bufferen som RGBA, rad for rad. */
    static void legg(BufferedImage b, int radius, ByteBuffer ut) {
        int w = FILM_BREDDE;
        int h = FILM_HOYDE;
        int[] p = new int[w * h];
        if (b.getWidth() == w && b.getHeight() == h) {
            b.getRGB(0, 0, w, h, p, 0, w);
        } else {
            BufferedImage s = new BufferedImage(w, h, BufferedImage.TYPE_INT_RGB);
            Graphics2D g = s.createGraphics();
            g.setRenderingHint(RenderingHints.KEY_INTERPOLATION, RenderingHints.VALUE_INTERPOLATION_BILINEAR);
            g.drawImage(b, 0, 0, w, h, null);
            g.dispose();
            s.getRGB(0, 0, w, h, p, 0, w);
        }
        if (radius > 0) {
            p = uskarp(p, w, h, radius);
        }
        ut.clear();
        for (int y = 0; y < h; y++) {
            ut.position(y * BREDDE * 4);
            for (int x = 0; x < w; x++) {
                int c = p[y * w + x];
                ut.put((byte) (c >> 16)).put((byte) (c >> 8)).put((byte) c).put((byte) 255);
            }
        }
        ut.position(0).limit(ut.capacity());
    }

    /** Boksuskarphet, vannrett og så loddrett. */
    static int[] uskarp(int[] inn, int w, int h, int r) {
        int[] mid = new int[inn.length];
        int[] ut = new int[inn.length];
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                mid[y * w + x] = snitt(inn, y * w, 1, Math.max(0, x - r), Math.min(w - 1, x + r));
            }
        }
        for (int x = 0; x < w; x++) {
            for (int y = 0; y < h; y++) {
                ut[y * w + x] = snitt(mid, x, w, Math.max(0, y - r), Math.min(h - 1, y + r));
            }
        }
        return ut;
    }

    private static int snitt(int[] p, int start, int steg, int fra, int til) {
        int rr = 0, gg = 0, bb = 0, n = til - fra + 1;
        for (int i = fra; i <= til; i++) {
            int c = p[start + i * steg];
            rr += (c >> 16) & 255;
            gg += (c >> 8) & 255;
            bb += c & 255;
        }
        return ((rr / n) << 16) | ((gg / n) << 8) | (bb / n);
    }
}
