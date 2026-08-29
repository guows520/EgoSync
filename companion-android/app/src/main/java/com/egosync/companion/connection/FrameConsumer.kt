package com.egosync.companion.connection

/**
 * 帧消费者注入缝（Story 13.2 T3）：会话循环收到的 SNAPSHOT/STATE_DELTA 帧经此
 * 交给快照通道（重组 → 解析 → StateMerger 全量替换）。
 *
 * - [ConnectionClient] 接口六成员签名零改动（UX-M2）；
 * - 消费者经 [RealConnectionClient] 构造注入（同 secrets/nsd/wsOpener/ioDispatcher
 *   注入缝手法，保持 JVM 可测）；
 * - **每会话一个实例**（`frameConsumerFactory()` 于 startSessionLoop 开头调用）：
 *   承载切换/重连时会话循环重建，残缺分帧序列不跨会话（对齐桌面「整序列持锁
 *   入队」语义）；
 * - 消费者内部自行吞掉载荷级错误（重组/解析失败记日志并重置）——载荷损坏
 *   不应杀死整条会话（下一个全量序列自愈）。
 */
fun interface FrameConsumer {
    fun onFrame(frame: Frame)

    companion object {
        val NOOP = FrameConsumer { _ -> }
    }
}
