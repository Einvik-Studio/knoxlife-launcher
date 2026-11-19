package knoxlife.agent;

import java.io.Closeable;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.channels.FileChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * En film til kinoen (J11): én {@code .knf}-fil per film. Lille-endian: {@code KNF1}, lengden på hodet (int32), hodet som
 * JSON ({@code id}, {@code bredde}, {@code hoyde}, {@code fps}, {@code bilder}, {@code lyd.bytes}), så
 * {@code bilder + 1} offset (int64, fra starten av filen – den siste er slutten av bildene), så bildene som JPEG og til
 * slutt lydsporet som OGG Vorbis. Filen lages av Knox Life fra originalen, og sjekksummen står i modets liste.
 */
public final class KinoFil implements Closeable {
    static final byte[] MERKE = "KNF1".getBytes(StandardCharsets.US_ASCII);
    /** Høyst så stort hode, og så mange bilder (tre timer med 24 bilder i sekundet). */
    static final int HODE_HOYST = 4096;
    static final int BILDER_HOYST = 24 * 60 * 60 * 3;

    public record Hode(String id, int bredde, int hoyde, double fps, int bilder, long lydBytes) {}

    private final FileChannel fil;
    private final Hode hode;
    private final long[] offset;

    private KinoFil(FileChannel fil, Hode hode, long[] offset) {
        this.fil = fil;
        this.hode = hode;
        this.offset = offset;
    }

    public static KinoFil åpne(Path sti) throws IOException {
        FileChannel f = FileChannel.open(sti, StandardOpenOption.READ);
        try {
            ByteBuffer b = les(f, 0, 8);
            for (int i = 0; i < 4; i++) {
                if (b.get(i) != MERKE[i]) {
                    throw new IOException("not a film file");
                }
            }
            int lengde = b.getInt(4);
            if (lengde <= 0 || lengde > HODE_HOYST) {
                throw new IOException("bad header");
            }
            String json = StandardCharsets.UTF_8.decode(les(f, 8, lengde)).toString();
            Hode h = tolk(json);
            ByteBuffer ob = les(f, 8L + lengde, (h.bilder() + 1) * 8);
            long[] o = new long[h.bilder() + 1];
            long størrelse = f.size();
            long forrige = 8L + lengde + (h.bilder() + 1) * 8L;
            for (int i = 0; i < o.length; i++) {
                o[i] = ob.getLong(i * 8);
                if (o[i] < forrige || o[i] > størrelse) {
                    throw new IOException("bad frame index");
                }
                forrige = o[i];
            }
            if (o[h.bilder()] + h.lydBytes() != størrelse) {
                throw new IOException("bad length");
            }
            return new KinoFil(f, h, o);
        } catch (IOException | RuntimeException e) {
            f.close();
            throw e;
        }
    }

    private static final Pattern TALL = Pattern.compile("\"(bredde|hoyde|fps|bilder|bytes)\"\\s*:\\s*([0-9.]+)");
    private static final Pattern ID = Pattern.compile("\"id\"\\s*:\\s*\"([a-z0-9_-]{1,40})\"");

    static Hode tolk(String json) throws IOException {
        Matcher mi = ID.matcher(json);
        if (!mi.find()) {
            throw new IOException("header without id");
        }
        double bredde = -1, hoyde = -1, fps = -1, bilder = -1, bytes = -1;
        Matcher m = TALL.matcher(json);
        while (m.find()) {
            double v = Double.parseDouble(m.group(2));
            switch (m.group(1)) {
                case "bredde" -> bredde = v;
                case "hoyde" -> hoyde = v;
                case "fps" -> fps = v;
                case "bilder" -> bilder = v;
                default -> bytes = v;
            }
        }
        if (bredde < 1 || bredde > 1024 || hoyde < 1 || hoyde > 512 || fps <= 0 || fps > 60 || bilder < 1
                || bilder > BILDER_HOYST || bytes < 0) {
            throw new IOException("bad header values");
        }
        return new Hode(mi.group(1), (int) bredde, (int) hoyde, fps, (int) bilder, (long) bytes);
    }

    private static ByteBuffer les(FileChannel f, long fra, int lengde) throws IOException {
        ByteBuffer b = ByteBuffer.allocate(lengde).order(ByteOrder.LITTLE_ENDIAN);
        while (b.hasRemaining()) {
            if (f.read(b, fra + b.position()) < 0) {
                throw new IOException("file too short");
            }
        }
        return b.flip();
    }

    public Hode hode() {
        return hode;
    }

    /** Lengden på filmen i sekunder. */
    public double sekunder() {
        return hode.bilder() / hode.fps();
    }

    /** Bildet nummer {@code i} som JPEG. */
    public byte[] bilde(int i) throws IOException {
        int n = Math.max(0, Math.min(hode.bilder() - 1, i));
        long fra = offset[n];
        long lengde = offset[n + 1] - fra;
        if (lengde > 4_000_000) {
            throw new IOException("frame too large");
        }
        return les(fil, fra, (int) lengde).array();
    }

    /** Skriver lydsporet til en egen fil, så spillets lydmotor kan strømme det. */
    public void lydTil(Path ut) throws IOException {
        Path del = ut.resolveSibling(ut.getFileName() + ".del");
        try (FileChannel u = FileChannel.open(del, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING,
                StandardOpenOption.WRITE)) {
            long fra = offset[hode.bilder()];
            long igjen = hode.lydBytes();
            while (igjen > 0) {
                long n = fil.transferTo(fra, igjen, u);
                if (n <= 0) {
                    throw new IOException("short audio");
                }
                fra += n;
                igjen -= n;
            }
        }
        Files.move(del, ut, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
    }

    @Override
    public void close() throws IOException {
        fil.close();
    }

    /** SHA-256 av hele filen, som heksadesimal med små bokstaver. */
    public static String sha256(Path sti) throws IOException {
        MessageDigest md;
        try {
            md = MessageDigest.getInstance("SHA-256");
        } catch (NoSuchAlgorithmException e) {
            throw new IOException(e);
        }
        try (InputStream inn = Files.newInputStream(sti)) {
            byte[] buf = new byte[1 << 16];
            for (int n; (n = inn.read(buf)) > 0;) {
                md.update(buf, 0, n);
            }
        }
        return HexFormat.of().formatHex(md.digest());
    }

    /** Lager en fil i formatet (prøvene, og skriptet som lager filmene, gjør det samme). */
    public static void skriv(Path ut, String id, int bredde, int hoyde, double fps, List<byte[]> bilder, byte[] lyd)
            throws IOException {
        String json = "{\"id\":\"" + id + "\",\"bredde\":" + bredde + ",\"hoyde\":" + hoyde + ",\"fps\":" + fps
                + ",\"bilder\":" + bilder.size() + ",\"lyd\":{\"format\":\"ogg\",\"bytes\":" + lyd.length + "}}";
        byte[] h = json.getBytes(StandardCharsets.UTF_8);
        long start = 8L + h.length + (bilder.size() + 1) * 8L;
        ByteBuffer o = ByteBuffer.allocate((bilder.size() + 1) * 8).order(ByteOrder.LITTLE_ENDIAN);
        long p = start;
        for (byte[] b : bilder) {
            o.putLong(p);
            p += b.length;
        }
        o.putLong(p);
        ByteBuffer topp = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).put(MERKE).putInt(h.length);
        try (var u = Files.newOutputStream(ut)) {
            u.write(topp.array());
            u.write(h);
            u.write(o.array());
            for (byte[] b : bilder) {
                u.write(b);
            }
            u.write(lyd);
        }
    }
}
