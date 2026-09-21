// Story 16.3：远程模式设置分区（GlobalSettingsModal「远程模式」tab）。
//
// 两态（进程内模式恒定——切换必经重启）：
// - **local 态**：远程实例 URL + 访问令牌 + 「测试连接」+「切换到远程模式」
//   （测试通过才可切——I/O 矩阵「测试失败→切换按钮禁用+原因」）；
//   切换 = 诚实代价确认对话框 → save（模式文件 + keyring）→ restart；
// - **remote 态**：连接信息（URL + 模式徽标）+「切回本地」
//   （确认 → save(mode=local) → restart——本地引擎以既有本地数据启动）。
//
// 诚实代价文案（AC 冻结款）：本地引擎完整停机（不产生本地数据副本）、
// 切换是连接目标变更（**永不合并/同步两侧数据**）、应用将重启。
//
// 令牌安全：只入 keyring（经壳命令 remote_mode_save_config——服务端/Rust
// 侧落盘），本组件不持久化任何令牌副本；本地态不回显既有令牌
// （desktop_get_boot_config 本地态不读 keyring——零回显面）。

import { FormEvent, useState } from 'react';
import { Cloud, Loader2, Check, AlertCircle, ArrowLeft, RefreshCw } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getTransportBoot } from '@/transport';
import { getDesktopMode } from '@/appMode';
import {
  remoteModeSaveConfig,
  remoteModeRestart,
} from '../../services/desktopModeService';

/** 测试连接结果（三态分流——网络错误≠401 严格分流的设置侧落点）。 */
type TestResult =
  | { kind: 'success'; message: string }
  | { kind: 'error'; message: string };

/** URL 形态预校验（与 Rust validate_remote_url 同源约定：http/https scheme）。 */
function validateUrlShape(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return '请输入远程实例地址';
  if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
    return '实例地址必须以 http:// 或 https:// 开头';
  }
  return null;
}

/**
 * 测试连接：`GET {url}/api/auth/status` 携候选令牌（Bearer 判定）。
 *
 * 直连 fetch（不经传输单例——单例在 local 态是 TauriTransport）。
 * 网络失败 ⇒ 明确「无法连接」文案（**不是**令牌问题——I/O 矩阵
 * 「网络错误≠401，不误报令牌失效」）。
 */
async function testConnection(url: string, token: string): Promise<TestResult> {
  const base = url.trim().replace(/\/+$/, '');
  let res: Response;
  try {
    res = await fetch(`${base}/api/auth/status`, {
      credentials: 'same-origin',
      headers: { Authorization: `Bearer ${token.trim()}` },
    });
  } catch {
    return { kind: 'error', message: '无法连接到实例，请检查地址与网络。' };
  }
  if (res.status === 429) {
    return { kind: 'error', message: '尝试过于频繁，请稍后再试。' };
  }
  if (res.status >= 500) {
    return { kind: 'error', message: '实例暂时不可用（服务端错误），请稍后再试。' };
  }
  if (res.status !== 200) {
    return { kind: 'error', message: `实例响应异常（HTTP ${res.status}），请检查地址。` };
  }
  let parsed: { setupRequired?: unknown; authenticated?: unknown };
  try {
    parsed = (await res.json()) as { setupRequired?: unknown; authenticated?: unknown };
  } catch {
    return { kind: 'error', message: '实例响应格式异常，请确认这是 EgoSync 实例。' };
  }
  if (parsed.authenticated === true) {
    return { kind: 'success', message: '连接成功：令牌有效。' };
  }
  if (parsed.setupRequired === true) {
    return {
      kind: 'error',
      message: '实例可达但尚未初始化：请先在浏览器中访问该实例完成初始化，再切换到远程模式。',
    };
  }
  return { kind: 'error', message: '令牌无效或权限不足，请核对后重试。' };
}

/** 诚实代价确认文案（AC 冻结款——切换语义的唯一说明面）。 */
const HONEST_COST_TEXT = [
  '切换到远程模式后：',
  '1. 本地引擎将完全停止（本次重启后不再启动）——桌面将成为该远程实例的客户端，与浏览器访问等价；',
  '2. 远程模式期间不会在本地写入任何业务数据；',
  '3. 切换只是更换连接目标，绝不会合并或同步两边的数据；',
  '4. 应用将重启进入远程模式；随时可在设置中切回本地（本地数据原样保留）。',
];

export function RemoteModeSection() {
  const mode = getDesktopMode();
  const bootUrl = getTransportBoot()?.remoteUrl ?? null;
  const isRemote = mode === 'remote';

  // local 态表单（URL 预填模式文件持久值；令牌不预填——零回显面）
  const [remoteUrl, setRemoteUrl] = useState(bootUrl ?? '');
  const [token, setToken] = useState('');
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);
  const [showSwitchConfirm, setShowSwitchConfirm] = useState(false);
  const [isSwitching, setIsSwitching] = useState(false);
  const [switchError, setSwitchError] = useState('');
  // remote 态切回本地
  const [showLocalConfirm, setShowLocalConfirm] = useState(false);
  const [isSwitchingBack, setIsSwitchingBack] = useState(false);
  const [switchBackError, setSwitchBackError] = useState('');

  const urlError = validateUrlShape(remoteUrl);
  const tokenTrimmed = token.trim();
  const canTest = !urlError && tokenTrimmed.length > 0 && !isTesting;
  const canSwitch = testResult?.kind === 'success';

  const handleTest = async () => {
    if (!canTest) return;
    setIsTesting(true);
    setTestResult(null);
    try {
      setTestResult(await testConnection(remoteUrl, tokenTrimmed));
    } finally {
      setIsTesting(false);
    }
  };

  const handleSwitchToRemote = async () => {
    if (!canSwitch || isSwitching) return;
    setIsSwitching(true);
    setSwitchError('');
    try {
      await remoteModeSaveConfig({ mode: 'remote', remoteUrl: remoteUrl.trim(), token: tokenTrimmed });
      await remoteModeRestart();
      // restart 不返回（进程退出重启）——此处实际不可达
    } catch (e) {
      setIsSwitching(false);
      setShowSwitchConfirm(false);
      setSwitchError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleSwitchToLocal = async () => {
    if (isSwitchingBack) return;
    setIsSwitchingBack(true);
    setSwitchBackError('');
    try {
      // remoteUrl 保留（下次切换预填——Rust 命令契约「local 态可保留」；
      // keyring 令牌同样保留：切回远程免重录）
      await remoteModeSaveConfig({ mode: 'local', remoteUrl: bootUrl });
      await remoteModeRestart();
    } catch (e) {
      setIsSwitchingBack(false);
      setShowLocalConfirm(false);
      setSwitchBackError(e instanceof Error ? e.message : String(e));
    }
  };

  // ── remote 态：连接信息 + 切回本地 ──
  if (isRemote) {
    return (
      <div className="space-y-6" data-testid="remote-mode-section" data-mode="remote">
        <div className="bg-indigo-50 dark:bg-indigo-900/30 border border-indigo-100 dark:border-indigo-700 rounded-xl p-4 text-[13px] text-indigo-800 dark:text-indigo-200 leading-relaxed">
          <div className="flex items-center gap-2 mb-2 font-medium">
            <Cloud size={16} className="shrink-0" />
            远程模式运行中
          </div>
          <p>桌面正在作为下方远程实例的客户端运行，与浏览器访问等价；本地引擎未启动，本地数据原样保留。</p>
        </div>

        <div>
          <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">
            远程实例地址
          </label>
          <div
            className="w-full bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-600 dark:text-slate-300 break-all"
            data-testid="remote-mode-current-url"
          >
            {bootUrl ?? '（模式文件未记录——切换时填写）'}
          </div>
          <p className="mt-2 text-[12px] text-slate-400 dark:text-slate-500">
            如需更换实例或令牌，先切回本地模式再重新配置（切换必经重启——模式在进程生命周期内恒定）。
          </p>
        </div>

        {switchBackError && (
          <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> 切回本地失败：{switchBackError}
          </div>
        )}

        {!showLocalConfirm ? (
          <button
            type="button"
            onClick={() => setShowLocalConfirm(true)}
            disabled={isSwitchingBack}
            className="inline-flex items-center gap-2 px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[14px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors disabled:opacity-50"
          >
            <ArrowLeft size={16} /> 切回本地模式
          </button>
        ) : (
          <div className="rounded-xl border border-amber-200 bg-amber-50/50 p-4 space-y-3">
            <p className="text-[13px] text-amber-700 leading-relaxed">
              确认切回本地模式？应用将重启并以本地引擎启动，使用本机既有的本地数据；远程实例数据不受影响，也不会做任何合并或同步。
            </p>
            <div className="flex items-center gap-2 pt-1">
              <button
                type="button"
                onClick={handleSwitchToLocal}
                disabled={isSwitchingBack}
                className="px-4 py-2 rounded-lg text-[13px] font-medium text-white bg-amber-600 hover:bg-amber-700 transition-colors disabled:opacity-50 inline-flex items-center gap-1.5"
              >
                {isSwitchingBack ? <Loader2 size={14} className="animate-loading-spin" /> : <RefreshCw size={14} />}
                {isSwitchingBack ? '正在重启...' : '确认并重启'}
              </button>
              <button
                type="button"
                onClick={() => { setShowLocalConfirm(false); setSwitchBackError(''); }}
                disabled={isSwitchingBack}
                className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
              >
                取消
              </button>
            </div>
          </div>
        )}
      </div>
    );
  }

  // ── local 态：配置 + 测试连接 + 切换 ──
  return (
    <div className="space-y-6" data-testid="remote-mode-section" data-mode="local">
      <div className="bg-indigo-50 dark:bg-indigo-900/30 border border-indigo-100 dark:border-indigo-700 rounded-xl p-4 text-[13px] text-indigo-800 dark:text-indigo-200 leading-relaxed">
        配置已部署的 EgoSync 远程实例：桌面将作为它的客户端运行（与浏览器访问等价），可随时切回本地模式。
      </div>

      <form
        className="space-y-4 text-left"
        noValidate
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          void handleTest();
        }}
      >
        <div>
          <label
            htmlFor="egosync-remote-url"
            className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
          >
            远程实例地址
          </label>
          <input
            id="egosync-remote-url"
            type="text"
            value={remoteUrl}
            onChange={e => {
              setRemoteUrl(e.target.value);
              setTestResult(null);
            }}
            placeholder="https://your-egosync-instance.example.com"
            autoComplete="off"
            spellCheck={false}
            disabled={isSwitching}
            className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
          />
          {urlError && remoteUrl.length > 0 && (
            <div className="mt-1.5 text-[12px] text-amber-600 dark:text-amber-400">{urlError}</div>
          )}
        </div>

        <div>
          <label
            htmlFor="egosync-remote-token"
            className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
          >
            访问令牌
          </label>
          <input
            id="egosync-remote-token"
            type="password"
            value={token}
            onChange={e => {
              setToken(e.target.value);
              setTestResult(null);
            }}
            placeholder="实例的访问令牌"
            autoComplete="new-password"
            disabled={isSwitching}
            className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
          />
          <p className="mt-1.5 text-[12px] text-slate-400 dark:text-slate-500">
            令牌只保存在系统钥匙串中，不写入磁盘明文。
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            type="submit"
            disabled={!canTest}
            className="inline-flex items-center gap-2 px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[14px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isTesting ? <Loader2 size={16} className="animate-loading-spin" /> : null}
            {isTesting ? '测试中...' : '测试连接'}
          </button>
          {testResult && (
            <div
              className={cn(
                'flex items-center gap-1.5 text-[13px]',
                testResult.kind === 'success' ? 'text-green-600' : 'text-red-600 dark:text-red-400',
              )}
              role="status"
              data-testid="remote-mode-test-result"
            >
              {testResult.kind === 'success' ? <Check size={14} /> : <AlertCircle size={14} />}
              <span>{testResult.message}</span>
            </div>
          )}
        </div>
      </form>

      {switchError && (
        <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
          <AlertCircle size={14} /> 切换失败：{switchError}
        </div>
      )}

      {!showSwitchConfirm ? (
        <div>
          <button
            type="button"
            onClick={() => setShowSwitchConfirm(true)}
            disabled={!canSwitch || isSwitching}
            title={canSwitch ? undefined : '请先完成测试连接且令牌有效'}
            className="inline-flex items-center gap-2 px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[14px] font-medium shadow-sm hover:bg-indigo-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Cloud size={16} /> 切换到远程模式
          </button>
          {!canSwitch && (
            <p className="mt-2 text-[12px] text-slate-400 dark:text-slate-500" data-testid="remote-mode-switch-hint">
              {testResult?.kind === 'error'
                ? `暂不可切换：${testResult.message}`
                : '填写实例地址与令牌并测试连接通过后，即可切换到远程模式。'}
            </p>
          )}
        </div>
      ) : (
        <div className="rounded-xl border border-amber-200 bg-amber-50/50 p-4 space-y-3" data-testid="remote-mode-switch-confirm">
          <div className="text-[13px] text-amber-800 leading-relaxed space-y-1">
            {HONEST_COST_TEXT.map((line, i) => (
              <p key={i}>{line}</p>
            ))}
          </div>
          <div className="flex items-center gap-2 pt-1">
            <button
              type="button"
              onClick={handleSwitchToRemote}
              disabled={isSwitching}
              className="px-4 py-2 rounded-lg text-[13px] font-medium text-white bg-amber-600 hover:bg-amber-700 transition-colors disabled:opacity-50 inline-flex items-center gap-1.5"
            >
              {isSwitching ? <Loader2 size={14} className="animate-loading-spin" /> : <RefreshCw size={14} />}
              {isSwitching ? '正在重启...' : '确认切换并重启'}
            </button>
            <button
              type="button"
              onClick={() => { setShowSwitchConfirm(false); setSwitchError(''); }}
              disabled={isSwitching}
              className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
            >
              取消
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
