import { Wifi, WifiOff, Loader2 } from 'lucide-react';
import { cn } from '../../lib/utils';
import { useConnectionState } from '../../hooks/useConnectionState';
import type { ConnectionState } from '@/transport';

/**
 * 连接状态指示器（Story 16.2）——NFR-C7 断线诚实明示的消费面。
 *
 * 三态呈现（I/O 矩阵 + Design Notes「连接状态形态」决策）：
 * - `online`：常驻低显著度徽标（双宿主同渲染，保持视觉零分叉；
 *   显著度低至不干扰主界面）；
 * - `connecting`：短暂态，与 online 同形但中性色（初始 connecting /
 *   登出后撤订阅回 connecting——不与 reconnecting 混淆）；
 * - `reconnecting`：显著横幅（页面顶部通栏 + 铃铛级视觉重量）+
 *   徽标变色——网络中断期间用户必须明确知道「重连中」，不静默。
 *
 * 退避与重连逻辑归传输层（架构④：应用层只管状态呈现）——本组件
 * 零副作用，纯渲染。
 */

const STATE_CONFIG: Record<ConnectionState, { badgeCls: string; label: string }> = {
  online: { badgeCls: 'text-emerald-500/80', label: '在线' },
  connecting: { badgeCls: 'text-slate-400', label: '连接中' },
  reconnecting: { badgeCls: 'text-amber-500', label: '重连中' },
};

const BADGE_ICON: Record<ConnectionState, typeof Wifi> = {
  online: Wifi,
  connecting: Loader2,
  reconnecting: WifiOff,
};

export function ConnectionStatus() {
  const state = useConnectionState();
  const Icon = BADGE_ICON[state];
  const { badgeCls, label } = STATE_CONFIG[state];

  // reconnecting 期间显著横幅：置顶通栏（pointer-events-none 防误吞点击
  // ——只提示不挡操作）。评审修复：z-[60] 确保不被通知面板（z-50）遮挡
  // ——横幅是断线期间的核心指示（NFR-C7），面板开着时也必须可见；
  // top 侧加安全区内边距（viewport-fit=cover 下不压入 iOS 刘海/灵动岛）。
  return (
    <>
      {state === 'reconnecting' && (
        <div
          role="status"
          aria-live="polite"
          data-testid="connection-banner"
          className="fixed top-0 pt-[max(0px,env(safe-area-inset-top))] inset-x-0 z-[60] flex items-center justify-center gap-2 px-4 py-2 bg-amber-50/95 dark:bg-amber-950/90 border-b border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-300 text-[13px] font-medium shadow-sm pointer-events-none"
        >
          <WifiOff size={14} className="shrink-0 animate-pulse motion-reduce:animate-none" />
          <span>连接已断开，正在重连…（恢复后将自动同步最新数据）</span>
        </div>
      )}
      <div
        data-testid="connection-status"
        data-state={state}
        role="status"
        aria-label={`连接状态：${label}`}
        title={`连接状态：${label}`}
        className={cn('flex items-center gap-1 text-[11px] font-medium select-none', badgeCls)}
      >
        <Icon
          size={12}
          className={cn(
            'shrink-0',
            state === 'connecting' && 'animate-spin motion-reduce:animate-none',
            state === 'reconnecting' && 'animate-pulse motion-reduce:animate-none',
          )}
        />
        <span className="hidden sm:inline">{label}</span>
      </div>
    </>
  );
}
