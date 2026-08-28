// 手机伴侣配对区（Story 12.2）——QR 展示、已配对设备卡、换绑确认、端口/防火墙说明。

import { useCallback, useEffect, useRef, useState } from 'react';
import { Smartphone, Trash2, Loader2, Check, AlertCircle, ShieldCheck, QrCode } from 'lucide-react';
import { cn } from '../../lib/utils';
import { companionService } from '../../services/companionService';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import type { CompanionStatus, PairedDevice, QrPayload } from '../../types/companion';

/** 与后端 PAIRING_WINDOW_TIMEOUT_SECS（300s）对齐的二维码有效期。 */
const QR_VALIDITY_MS = 300_000;

function toFriendlyError(e: unknown, fallback: string): string {
  if (typeof e === 'object' && e !== null) {
    const first = Object.values(e)[0];
    if (typeof first === 'string') return first;
  }
  if (typeof e === 'string') return e;
  return fallback;
}

function formatCountdown(remainingMs: number): string {
  const totalSec = Math.max(0, Math.ceil(remainingMs / 1000));
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function CompanionPairingSection() {
  const [qrPayload, setQrPayload] = useState<QrPayload | null>(null);
  const [qrSvg, setQrSvg] = useState<string>('');
  const [isLoadingQr, setIsLoadingQr] = useState(false);
  const [qrExpiresAt, setQrExpiresAt] = useState<number | null>(null);
  const [nowTick, setNowTick] = useState(Date.now());
  const [devices, setDevices] = useState<PairedDevice[]>([]);
  const [status, setStatus] = useState<CompanionStatus | null>(null);
  const [error, setError] = useState('');
  const [loadError, setLoadError] = useState('');
  const [isConfirming, setIsConfirming] = useState(false);
  const [removingDevice, setRemovingDevice] = useState<PairedDevice | null>(null);
  const [isRemoving, setIsRemoving] = useState(false);
  const [confirmedName, setConfirmedName] = useState<string | null>(null);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [deviceList, currentStatus] = await Promise.all([
        companionService.listPairedDevices(),
        companionService.getStatus(),
      ]);
      setDevices(deviceList);
      setStatus(currentStatus);
      setLoadError('');
    } catch (e) {
      console.error('加载手机伴侣状态失败:', e);
      // keyring 不可用等持续性故障必须对用户可见（否则配对入口静默死区）
      setLoadError(toFriendlyError(e, '手机伴侣状态加载失败（系统钥匙串可能不可用），请检查钥匙串权限或重启应用'));
    }
  }, []);

  useEffect(() => {
    refresh();
    // modal 打开期间轮询状态（换绑 pending 可见性）
    pollTimerRef.current = setInterval(refresh, 3000);
    return () => {
      if (pollTimerRef.current) clearInterval(pollTimerRef.current);
    };
  }, [refresh]);

  useTauriEvent<{ deviceId: string }>('companion:paired', () => {
    refresh();
  }, [refresh]);

  useTauriEvent<{ deviceId: string }>('companion:connected', () => {
    refresh();
  }, [refresh]);

  useTauriEvent<{ deviceId: string }>('companion:disconnected', () => {
    refresh();
  }, [refresh]);

  // 二维码有效期倒计时（与后端 300s 配对窗口对齐）
  useEffect(() => {
    if (qrExpiresAt === null) return;
    const timer = setInterval(() => setNowTick(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [qrExpiresAt]);

  // 过期自动收回：超时后扫码会被后端静默拒绝，必须让用户看到码已失效
  useEffect(() => {
    if (qrExpiresAt !== null && nowTick >= qrExpiresAt) {
      setQrPayload(null);
      setQrSvg('');
      setQrExpiresAt(null);
    }
  }, [nowTick, qrExpiresAt]);

  // 新换绑请求出现时清除旧的换绑成功提示（成功提示不得跨请求残留）
  const pendingPubkey = status?.pendingPairing?.devicePubkey ?? null;
  useEffect(() => {
    if (pendingPubkey) setConfirmedName(null);
  }, [pendingPubkey]);

  const handleGenerateQr = async () => {
    setIsLoadingQr(true);
    setError('');
    try {
      const payload = await companionService.generateQr();
      const svg = await companionService.renderQrSvg(JSON.stringify(payload));
      setQrPayload(payload);
      setQrSvg(svg);
      setQrExpiresAt(Date.now() + QR_VALIDITY_MS);
    } catch (e) {
      setError(toFriendlyError(e, '二维码生成失败，请稍后重试'));
    } finally {
      setIsLoadingQr(false);
    }
  };

  const handleConfirmPairing = async () => {
    setIsConfirming(true);
    setError('');
    setConfirmedName(null);
    try {
      const device = await companionService.confirmPairing();
      setConfirmedName(device.deviceName);
      await refresh();
    } catch (e) {
      setError(toFriendlyError(e, '确认配对失败，请稍后重试'));
    } finally {
      setIsConfirming(false);
    }
  };

  const handleRemoveDevice = async () => {
    if (!removingDevice) return;
    setIsRemoving(true);
    setError('');
    try {
      await companionService.removePairedDevice(removingDevice.id);
      setRemovingDevice(null);
      await refresh();
    } catch (e) {
      setError(toFriendlyError(e, '移除配对失败，请稍后重试'));
    } finally {
      setIsRemoving(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="bg-indigo-50 dark:bg-indigo-900/30 border border-indigo-100 dark:border-indigo-700 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed flex items-start gap-2">
        <Smartphone size={16} className="mt-0.5 shrink-0" />
        <span>生成配对二维码后，用手机扫码即可绑定。配对一次后，手机在同一局域网内可自动发现并免扫码重连。当前版本仅支持局域网直连，中继服务暂未部署。</span>
      </div>

      {error && (
        <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
          <AlertCircle size={14} /> {error}
        </div>
      )}

      {loadError && (
        <div className="rounded-lg border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-3 py-2 text-[13px] text-amber-700 dark:text-amber-300 flex items-center gap-2">
          <AlertCircle size={14} /> {loadError}
        </div>
      )}

      {status?.pendingPairing && (
        <div className="rounded-xl border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 p-4">
          <div className="flex items-center justify-between gap-4">
            <div className="min-w-0">
              <p className="text-[14px] font-medium text-amber-900 dark:text-amber-200">新设备请求替换配对</p>
              <p className="mt-1 text-[13px] text-amber-800 dark:text-amber-300">
                「{status.pendingPairing.deviceName}」请求成为新的配对手机（确认后旧设备将被解绑，120 秒内有效）。
              </p>
            </div>
            <button
              onClick={handleConfirmPairing}
              disabled={isConfirming}
              className="shrink-0 px-4 py-2 rounded-lg bg-amber-600 text-white text-[13px] font-medium hover:bg-amber-700 transition-colors disabled:opacity-50 flex items-center gap-1.5"
            >
              {isConfirming ? <Loader2 size={14} className="animate-loading-spin" /> : <Check size={14} />}
              确认换绑
            </button>
          </div>
        </div>
      )}

      {confirmedName && (
        <div className="rounded-lg border border-green-100 bg-green-50 px-3 py-2 text-[13px] text-green-700 flex items-center gap-2">
          <Check size={14} /> 已完成换绑：{confirmedName}
        </div>
      )}

      {/* QR 生成与展示 */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl p-5 shadow-sm">
        <div className="flex items-center gap-2 mb-3">
          <QrCode size={16} className="text-slate-500 dark:text-slate-400" />
          <h4 className="text-[15px] font-medium text-slate-800 dark:text-slate-100">配对二维码</h4>
        </div>
        {qrPayload ? (
          <div className="flex flex-col sm:flex-row gap-5 items-start">
            <div
              className="w-[220px] h-[220px] shrink-0 rounded-lg border border-slate-200 dark:border-slate-700 bg-white p-2 [&>svg]:w-full [&>svg]:h-full"
              dangerouslySetInnerHTML={{ __html: qrSvg }}
              role="img"
              aria-label="手机伴侣配对二维码"
            />
            <div className="min-w-0 text-[13px] text-slate-500 dark:text-slate-400 space-y-1.5">
              <p>请用手机伴侣 App 扫描此二维码完成配对。二维码单次有效，重新生成会使旧码失效。</p>
              {qrExpiresAt !== null && (
                <p className="text-amber-600 dark:text-amber-400">
                  二维码 {formatCountdown(qrExpiresAt - nowTick)} 后失效，超时需重新生成。
                </p>
              )}
              <p className="text-slate-400 dark:text-slate-500">中继未部署——手机需与电脑处于同一局域网。</p>
              {status?.port ? (
                <p className="text-slate-400 dark:text-slate-500">连接端口（动态分配）：{status.port}</p>
              ) : null}
            </div>
          </div>
        ) : (
          <button
            onClick={handleGenerateQr}
            disabled={isLoadingQr}
            className="w-full py-4 border-2 border-dashed border-slate-200 dark:border-slate-700 rounded-xl text-[14px] font-medium text-slate-500 dark:text-slate-400 hover:border-indigo-300 dark:hover:border-indigo-600 hover:text-indigo-600 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/30 transition-all flex items-center justify-center gap-2 disabled:opacity-50"
          >
            {isLoadingQr ? <Loader2 size={16} className="animate-loading-spin" /> : <QrCode size={16} />}
            生成配对二维码
          </button>
        )}
      </div>

      {/* 已配对设备 */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl p-5 shadow-sm">
        <h4 className="text-[15px] font-medium text-slate-800 dark:text-slate-100 mb-3">已配对设备</h4>
        {devices.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-4 py-6 text-center text-[13px] text-slate-400 dark:text-slate-500">
            暂未配对任何手机
          </div>
        ) : devices.map((device) => {
          const isConnected = status?.connected?.deviceId === device.id;
          return (
            <div key={device.id} className="flex items-start justify-between gap-4 py-3 border-b border-slate-100 dark:border-slate-800 last:border-0 last:pb-0 first:pt-0">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className={cn('w-2.5 h-2.5 rounded-full', isConnected ? 'bg-emerald-500' : 'bg-slate-300')} />
                  <span className="text-[14px] font-medium text-slate-800 dark:text-slate-100 break-words">{device.deviceName}</span>
                  {isConnected && <span className="rounded-full border border-emerald-200 dark:border-emerald-700 bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-300">已连接</span>}
                </div>
                <p className="mt-1 font-mono text-[12px] text-slate-400 dark:text-slate-500">{device.devicePubkey.slice(0, 16)}…</p>
                <p className="mt-0.5 text-[12px] text-slate-400 dark:text-slate-500">最后在线：{device.lastSeenAt}</p>
              </div>
              <button
                onClick={() => setRemovingDevice(device)}
                className="shrink-0 px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30 rounded-lg transition-colors"
              >
                移除
              </button>
            </div>
          );
        })}
      </div>

      {/* 端口策略与防火墙说明（AC5） */}
      <div className="bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 text-[13px] text-slate-500 dark:text-slate-400 leading-relaxed">
        <div className="flex items-center gap-2 mb-2 text-slate-600 dark:text-slate-300">
          <ShieldCheck size={14} />
          <span className="font-medium">连接说明</span>
        </div>
        <p>桌面通过局域网发现服务（mDNS，服务名 <span className="font-mono">_egosync._tcp</span>）自动广播连接端口，端口由系统动态分配，无需手动配置。</p>
        <p className="mt-1.5">首次使用时，Windows 防火墙可能弹出「允许 EgoSync 访问网络」的提示——请勾选「专用网络」并允许，否则手机无法发现电脑。</p>
        <p className="mt-1.5">移除配对后，该手机将立即失去连接资格，重新配对需再次扫码。</p>
      </div>

      {/* 移除确认 */}
      {removingDevice && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-900/30 px-4">
          <div className="w-full max-w-md rounded-2xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 p-6 shadow-2xl">
            <h4 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100">确认移除配对？</h4>
            <p className="mt-2 text-[13px] leading-relaxed text-slate-500 dark:text-slate-400">
              移除后「{removingDevice.deviceName}」将无法再连接这台电脑，需要重新扫码配对。
            </p>
            <div className="mt-6 flex justify-end gap-3">
              <button onClick={() => setRemovingDevice(null)} disabled={isRemoving} className="px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 text-[13px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors">取消</button>
              <button onClick={handleRemoveDevice} disabled={isRemoving} className="px-4 py-2 rounded-lg bg-red-600 text-[13px] font-medium text-white hover:bg-red-700 transition-colors disabled:opacity-50 flex items-center gap-1.5">
                {isRemoving ? <Loader2 size={14} className="animate-loading-spin" /> : <Trash2 size={14} />}
                确认移除
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
