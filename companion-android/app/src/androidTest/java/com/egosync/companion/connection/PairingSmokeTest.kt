package com.egosync.companion.connection

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 配对冒烟（Story 12.4 Task 8，AC7）——在真机/模拟器上验证 JVM 单测覆盖不到的
 * 设备真实路径：
 *
 * 1. **真 Keystore 路径**：[PairingSecrets] 的 X25519 静态私钥生成 →
 *    Android Keystore AES-GCM 包裹落盘 → 重新加载，字节一致（wrap 布局/主线程
 *    约束/设备密钥库实现只有真机能暴露）；
 * 2. **on-device noise-java 互通**：initiator（手机侧，用 Keystore 私钥）与
 *    responder（进程内「桌面小服务器」）完成 Noise XX 三消息握手——12.1 黄金
 *    向量在 JVM 验过逐字节一致，此处在 Android 运行时再验一次库可加载可用；
 * 3. **帧序列往返**：HELLO → NOTICE(deviceInfo) → NOTICE(pairingAuth) →
 *    PING/PONG，双端加解密密文经通道桥互通（桌面语义：PING 应答同为 Ping 帧）。
 *
 * NSD 发现 / 真 WS socket / 与桌面端到端互通属手动冒烟范围（需双设备局域网），
 * 不在本测试内，验证边界见 story Dev Agent Record。
 */
@RunWith(AndroidJUnit4::class)
class PairingSmokeTest {

    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    @Test
    fun keystoreWrapReloadAndNoiseHandshakeRoundtrip() = runBlocking {
        // ── 1. 真 Keystore 路径：生成 → 包裹落盘 → 重载一致 ──
        val secrets = PairingSecrets(context)
        secrets.wipe()
        val phonePriv = secrets.loadOrCreateStaticPrivateKey()
        assertEquals("X25519 私钥必须 32 字节", 32, phonePriv.size)
        val reloaded = secrets.loadOrCreateStaticPrivateKey()
        assertArrayEquals("重载私钥必须字节一致（wrap/unwrap 正确性）", phonePriv, reloaded)
        secrets.wipe()

        // ── 2. 进程内「桌面小服务器」：responder 侧 noise 泵 ──
        // 手机（initiator，Keystore 私钥）→ 通道桥 → 桌面（responder，独立密钥）
        val desktopPriv = ByteArray(32) { (it + 1).toByte() }
        val phoneToDesktop = Channel<ByteArray>(capacity = 4)
        val desktopToPhone = Channel<ByteArray>(capacity = 4)

        val desktopDone = CompletableDeferred<Unit>()
        val desktopScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val serverJob = desktopScope.launch {
            val responder = NoiseChannel.responder(desktopPriv)
            // XX responder：m1 入 → m2 出 → m3 入 → split
            responder.readHandshakeMessage(phoneToDesktop.receive())
            desktopToPhone.send(responder.writeHandshakeMessage())
            responder.readHandshakeMessage(phoneToDesktop.receive())
            val transport = responder.split()

            // 桌面首帧校验 + app 层 Notice 收集 + PING 应答（镜像 companion_connection.rs）
            val hello = FrameCodec.decode(phoneToDesktop.receive(), transport)
            assertTrue("首帧必须 HELLO", hello is Frame.Hello)
            var deviceName: String? = null
            var pairingNonce: String? = null
            while (deviceName == null || pairingNonce == null) {
                val notice = FrameCodec.decode(phoneToDesktop.receive(), transport)
                if (notice is Frame.Notice) {
                    val text = notice.data
                    if (text.contains("deviceInfo")) deviceName = text
                    if (text.contains("pairingAuth")) pairingNonce = text
                }
            }
            assertTrue("deviceInfo 必须到达", deviceName!!.contains("deviceName"))
            assertTrue("pairingAuth 必须携带 nonce", pairingNonce!!.contains("nonce"))

            // PING → PONG（桌面以 Ping 帧应答，12.2 语义）
            val ping = FrameCodec.decode(phoneToDesktop.receive(), transport)
            assertTrue("应收到 PING", ping is Frame.Ping)
            desktopToPhone.send(FrameCodec.encode(Frame.Ping, transport))
            desktopDone.complete(Unit)
        }

        // ── 3. 手机侧：真实握手 + 帧序列（复用 NoiseChannel/FrameCodec）──
        withTimeout(20_000) {
            val initiator = NoiseChannel.initiator(phonePriv)
            phoneToDesktop.send(initiator.writeHandshakeMessage())
            initiator.readHandshakeMessage(desktopToPhone.receive())
            phoneToDesktop.send(initiator.writeHandshakeMessage())
            val transport = initiator.split()

            // 与 RealConnectionClient.sendIntroFrames 同构的帧序
            phoneToDesktop.send(
                FrameCodec.encode(Frame.Hello(FrameCodec.PROTOCOL_VERSION), transport),
            )
            val deviceInfo = org.json.JSONObject()
                .put("type", "deviceInfo")
                .put("deviceName", "冒烟测试机")
                .toString()
            phoneToDesktop.send(FrameCodec.encode(Frame.Notice(deviceInfo), transport))
            val pairingAuth = org.json.JSONObject()
                .put("type", "pairingAuth")
                .put("nonce", "nonce-smoke")
                .toString()
            phoneToDesktop.send(FrameCodec.encode(Frame.Notice(pairingAuth), transport))

            // PING 往返：桌面应答 Ping 帧
            phoneToDesktop.send(FrameCodec.encode(Frame.Ping, transport))
            val pong = FrameCodec.decode(desktopToPhone.receive(), transport)
            assertTrue("PONG 必须是 Ping 帧（桌面应答语义）", pong is Frame.Ping)
        }

        // P19：join 必须有界——桌面侧泵若因缺帧卡在 receive()，无超时的 join
        // 会把「测试失败」变成「测试挂起」（CI 超时掩盖真实失败点）
        withTimeout(30_000) { serverJob.join() }
        assertTrue("桌面侧泵必须完成全部校验", desktopDone.isCompleted)
        desktopScope.coroutineContext[Job]?.cancel()
    }
}
