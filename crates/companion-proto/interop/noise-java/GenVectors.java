import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HexFormat;

import com.southernstorm.noise.protocol.CipherState;
import com.southernstorm.noise.protocol.CipherStatePair;
import com.southernstorm.noise.protocol.DHState;
import com.southernstorm.noise.protocol.HandshakeState;

/**
 * noise-java 侧黄金向量生成器（开发期手动运行，产物随测试提交）。
 *
 * 与 snow 响应方（crates/companion-proto/examples/gen_snow_responder.rs）经行协议
 * 完成真实的 Noise_XX_25519_ChaChaPoly_BLAKE2s 跨语言握手与传输期互验，
 * 产出 tests/fixtures/noise_java_vectors.json。
 *
 * 运行方式见同目录 run.sh / README.md；参数为 companion-proto crate 根目录路径。
 */
public class GenVectors {
    private static final String SUITE = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

    // 固定密钥（仅用于黄金向量生成，非生产密钥；响应方密钥由 rust 侧广播）
    private static final String INITIATOR_STATIC_HEX =
            "4c1b8e7a2d95f30618c4b7e9a3f5d0286b1e94c7a02d5f8b3e691c4a7d0f5b82";
    private static final String INITIATOR_EPHEMERAL_HEX =
            "e2815d0a7f3c9b46d195e02a8c7f3b64095e1d2c8a6f0b743e591d0c6a2f8e34";

    private static final HexFormat HEX = HexFormat.of();

    // 覆盖 8 种帧类型的明文（对 noise-java 为 opaque 字节；rust 测试侧按帧解码）
    private static final String[] PLAINTEXTS = {
            "{\"type\":\"hello\",\"protocolVersion\":1}",
            "{\"type\":\"snapshot\",\"data\":\"snapshot-全量快照\"}",
            "{\"type\":\"state_delta\",\"data\":\"delta-增量-1\"}",
            "{\"type\":\"command\",\"data\":\"cmd-1\"}",
            "{\"type\":\"command_result\",\"data\":\"result-ok\"}",
            "{\"type\":\"stream_token\",\"data\":\"token-1\"}",
            "{\"type\":\"notice\",\"data\":\"notice-1\"}",
            "{\"type\":\"ping\"}",
    };

    public static void main(String[] args) throws Exception {
        Path crateRoot = Path.of(args.length > 0 ? args[0] : ".").toAbsolutePath();

        ProcessBuilder pb = new ProcessBuilder(
                "cargo", "run", "--quiet", "--example", "gen_snow_responder");
        pb.directory(crateRoot.toFile());
        // cargo 编译/运行日志走 stderr——必须继承到本进程，否则管道写满导致生成器挂起。
        pb.redirectError(ProcessBuilder.Redirect.INHERIT);
        Process snow = pb.start();

        BufferedReader in = new BufferedReader(
                new InputStreamReader(snow.getInputStream(), StandardCharsets.UTF_8));
        PrintWriter out = new PrintWriter(
                new OutputStreamWriter(snow.getOutputStream(), StandardCharsets.UTF_8), true);

        // 1. 读取响应方固定密钥
        String keysLine = in.readLine();
        if (keysLine == null || !keysLine.startsWith("KEYS ")) {
            throw new IllegalStateException("snow 响应方未广播密钥: " + keysLine);
        }
        String[] keys = keysLine.substring(5).trim().split("\\s+");
        String responderStaticHex = keys[0];
        String responderEphemeralHex = keys[1];

        // 2. XX 握手（noise-java 为发起方）
        HandshakeState hs = new HandshakeState(SUITE, HandshakeState.INITIATOR);
        DHState local = hs.getLocalKeyPair();
        local.setPrivateKey(HEX.parseHex(INITIATOR_STATIC_HEX), 0);
        hs.getFixedEphemeralKey().setPrivateKey(HEX.parseHex(INITIATOR_EPHEMERAL_HEX), 0);
        hs.start();

        byte[] message = new byte[65535];
        byte[] payload = new byte[65535];

        int m1Len = hs.writeMessage(message, 0, payload, 0, 0);
        String m1 = HEX.formatHex(message, 0, m1Len);
        out.println(m1);
        String m2Raw = in.readLine();
        if (m2Raw == null) {
            throw new IllegalStateException("snow 响应方在握手第二步（写 m2）前断流");
        }
        String m2 = m2Raw.trim();
        hs.readMessage(HEX.parseHex(m2), 0, m2.length() / 2, payload, 0);
        int m3Len = hs.writeMessage(message, 0, payload, 0, 0);
        String m3 = HEX.formatHex(message, 0, m3Len);
        out.println(m3);
        String okLine = in.readLine();
        if (okLine == null || !"HANDSHAKE_OK".equals(okLine.trim())) {
            throw new IllegalStateException("snow 响应方握手未确认完成（响应: " + okLine + "）");
        }
        if (hs.getAction() != HandshakeState.SPLIT) {
            throw new IllegalStateException("noise-java 握手未完成（action=" + hs.getAction() + "）");
        }

        CipherStatePair pair = hs.split();
        CipherState sender = pair.getSender();
        CipherState receiver = pair.getReceiver();

        // 3. 传输期互验（先全部 i2r 再全部 r2i，与测试重放的 nonce 顺序一致）
        ArrayList<String> entries = new ArrayList<>();
        boolean snowDecryptedJavaOk = true;
        boolean javaDecryptedSnowOk = true;

        for (String plaintext : PLAINTEXTS) {
            byte[] pt = plaintext.getBytes(StandardCharsets.UTF_8);
            String ptHex = HEX.formatHex(pt);
            // i2r：noise-java 加密 → snow 解密并回显，验证一致
            int ctLen = sender.encryptWithAd(null, pt, 0, message, 0, pt.length);
            String ctHex = HEX.formatHex(message, 0, ctLen);
            out.println("DEC " + ctHex);
            String decResRaw = in.readLine();
            if (decResRaw == null) {
                throw new IllegalStateException("snow 响应方在 i2r 解密阶段断流（明文: " + ptHex + "）");
            }
            String decRes = decResRaw.trim();
            if (!decRes.equals("DECRES " + ptHex)) {
                snowDecryptedJavaOk = false;
            }
            entries.add(String.format(
                    "{\"direction\": \"i2r\", \"plaintext\": \"%s\", \"ciphertext\": \"%s\"}",
                    ptHex, ctHex));
        }
        for (String plaintext : PLAINTEXTS) {
            byte[] pt = plaintext.getBytes(StandardCharsets.UTF_8);
            String ptHex = HEX.formatHex(pt);
            // r2i：snow 加密回显密文 → noise-java 解密，验证一致
            out.println("ENC " + ptHex);
            String encResRaw = in.readLine();
            if (encResRaw == null) {
                throw new IllegalStateException("snow 响应方在 r2i 加密阶段断流（明文: " + ptHex + "）");
            }
            String encRes = encResRaw.trim();
            if (!encRes.startsWith("ENCRES ")) {
                throw new IllegalStateException("snow 响应方加密响应异常: " + encRes);
            }
            byte[] ct = HEX.parseHex(encRes.substring(7));
            String ctHex = HEX.formatHex(ct);
            int ptLen = receiver.decryptWithAd(null, ct, 0, payload, 0, ct.length);
            if (ptLen != pt.length
                    || !HEX.formatHex(payload, 0, ptLen).equals(ptHex)) {
                javaDecryptedSnowOk = false;
            }
            entries.add(String.format(
                    "{\"direction\": \"r2i\", \"plaintext\": \"%s\", \"ciphertext\": \"%s\"}",
                    ptHex, ctHex));
        }
        out.println("QUIT");
        int exitCode = snow.waitFor();
        if (exitCode != 0) {
            throw new IllegalStateException("snow 响应方退出码异常: " + exitCode);
        }
        // 显式失败：生成期互验不过，禁止产出向量文件
        if (!snowDecryptedJavaOk || !javaDecryptedSnowOk) {
            throw new IllegalStateException(String.format(
                    "生成期互验失败: snowDecryptedJava=%s, javaDecryptedSnow=%s",
                    snowDecryptedJavaOk, javaDecryptedSnowOk));
        }

        // 4. 写向量文件
        StringBuilder sb = new StringBuilder();
        sb.append("{\n");
        sb.append("  \"suite\": \"").append(SUITE).append("\",\n");
        sb.append("  \"generator\": \"noise-java (rweather, JitPack 49377b6dfc) via interop/noise-java/GenVectors.java\",\n");
        sb.append("  \"initiator\": {\n");
        sb.append("    \"staticPrivate\": \"").append(INITIATOR_STATIC_HEX).append("\",\n");
        sb.append("    \"ephemeralPrivate\": \"").append(INITIATOR_EPHEMERAL_HEX).append("\"\n");
        sb.append("  },\n");
        sb.append("  \"responder\": {\n");
        sb.append("    \"staticPrivate\": \"").append(responderStaticHex).append("\",\n");
        sb.append("    \"ephemeralPrivate\": \"").append(responderEphemeralHex).append("\"\n");
        sb.append("  },\n");
        sb.append("  \"handshakeMessages\": [\"").append(m1).append("\", \"")
                .append(m2).append("\", \"").append(m3).append("\"],\n");
        sb.append("  \"transport\": [\n    ");
        sb.append(String.join(",\n    ", entries));
        sb.append("\n  ],\n");
        sb.append("  \"generationTimeVerification\": {\n");
        sb.append("    \"snowDecryptedJavaCiphertexts\": true,\n");
        sb.append("    \"javaDecryptedSnowCiphertexts\": true\n");
        sb.append("  }\n");
        sb.append("}\n");

        Files.createDirectories(crateRoot.resolve("tests/fixtures"));
        Path output = crateRoot.resolve("tests/fixtures/noise_java_vectors.json");
        Files.writeString(output, sb.toString());
        System.out.println("已生成 " + output);
    }
}
