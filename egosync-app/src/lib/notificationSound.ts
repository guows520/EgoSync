/// Story 4.5: 敲门通知提示音（Web Audio API，无额外依赖）
/// 仅在用户于设置中开启 `notification.knock_sound` 时由调用方触发。

export function playNotificationSound(): void {
  try {
    const AudioCtor =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!AudioCtor) return;

    const ctx = new AudioCtor();
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    oscillator.connect(gainNode);
    gainNode.connect(ctx.destination);
    oscillator.frequency.value = 880; // A5
    gainNode.gain.setValueAtTime(0.1, ctx.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
    oscillator.start(ctx.currentTime);
    oscillator.stop(ctx.currentTime + 0.3);
    oscillator.onended = () => {
      void ctx.close();
    };
  } catch (e) {
    console.error('播放敲门提示音失败:', e);
  }
}
