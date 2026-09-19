// Story 15.5：工件全量驱动的双通道对等套件（「同参数经双通道响应 JSON
// 逐字节同构（含 AppError 形状）」的机制化）。
//
// 驱动源：crates/egosync-engine/commands.json 全部 103 条 web-ok 命令——
// 测试循环遍历工件条目，按参数 schema 机械生成最小参数 fixture
// （String→"x"、数值→1、bool→true、Option→省略、Vec→[]、结构体→{}），
// 测试代码内零手写命令名。
//
// 双通道：
// - Tauri 通道：mock @tauri-apps/api/core 的 invoke（resolve fixture /
//   reject AppError map）；
// - HTTP 通道：mock global fetch（200+body / 200+判别头+body / 401 等）。
//
// server 侧消费同一工件的 cargo 套件见 server/tests/command_parity_test.rs。

import { readFileSync } from 'node:fs';
import * as nodePath from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Tauri 通道 invoke 桩（vi.hoisted 供提升后的 mock 工厂引用）
const tauriInvokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauriInvokeMock }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: () => Promise.resolve(() => {}),
  emit: vi.fn(),
}));

import { HttpTransport } from './http';
import { TauriTransport } from './tauri';
import { HttpTransportError } from './types';

// vitest 运行 cwd = egosync-app（jsdom 全局 URL 补丁使 import.meta.url 相对解析不可用）
const repoRoot = nodePath.resolve(process.cwd(), '..');

interface ArtifactParam {
  name: string;
  camelName: string;
  type: string;
  kind: 'ctx-injected' | 'client';
}

interface ArtifactCommand {
  name: string;
  module: string;
  capability: string;
  params: ArtifactParam[];
}

/** commands.json 工件（对等用例的唯一驱动源）。 */
const artifact = JSON.parse(
  readFileSync(`${repoRoot}/crates/egosync-engine/commands.json`, 'utf8')
) as {
  version: number;
  count: number;
  commands: ArtifactCommand[];
  replayWhitelist: string[];
};

const commands = artifact.commands;

/** 按参数 schema 机械生成最小参数（现状惯例：单层扁平 camelCase）。 */
function minimalArgs(entry: ArtifactCommand): Record<string, unknown> | undefined {
  const args: Record<string, unknown> = {};
  for (const param of entry.params) {
    if (param.kind !== 'client') continue;
    const t = param.type;
    if (t.startsWith('Option<')) continue; // 可选 → 省略（服务层 `?? null` 惯例的缺省侧）
    if (t === 'String') args[param.camelName] = 'x';
    else if (/^(i|u|f)(8|16|32|64|128|size)?$/.test(t)) args[param.camelName] = 1;
    else if (t === 'bool') args[param.camelName] = true;
    else if (t.startsWith('Vec<')) args[param.camelName] = [];
    else args[param.camelName] = {}; // 结构体复合输入（包 `input` 键的现状惯例由 schema 决定）
  }
  return Object.keys(args).length > 0 ? args : undefined;
}

/** 确定性响应 fixture（双通道共享同一对象——逐字节同构天然满足）。 */
function responseFixture(command: string): { command: string; ok: boolean } {
  return { command, ok: true };
}

/** 假 fetch Response（transport 只消费 status / headers.get / text）。 */
function jsonResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  const text = JSON.stringify(body);
  const lower = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    status,
    headers: { get: (name: string) => lower.get(name.toLowerCase()) ?? null },
    text: () => Promise.resolve(text),
  } as unknown as Response;
}

/** error.rs 源码机械枚举 AppError variant 名（与 api_test 同口径）。 */
function extractAppErrorVariants(): string[] {
  const src = readFileSync(`${repoRoot}/crates/egosync-engine/src/error.rs`, 'utf8');
  const start = src.indexOf('pub enum AppError {');
  if (start < 0) throw new Error('error.rs 未找到 pub enum AppError {（格式变化？）');
  const rest = src.slice(start + 'pub enum AppError {'.length);
  const end = rest.indexOf('\n}');
  if (end < 0) throw new Error('error.rs AppError 枚举块未闭合（格式变化？）');
  const variants: string[] = [];
  for (const line of rest.slice(0, end).split('\n')) {
    const t = line.trim();
    let i = 0;
    while (i < t.length && /[A-Za-z0-9_]/.test(t[i]!)) i++;
    const ident = t.slice(0, i);
    if (ident && /^[A-Z]/.test(ident) && t.slice(ident.length).startsWith('(')) {
      variants.push(ident);
    }
  }
  return variants;
}

const variants = extractAppErrorVariants();

describe(`commands.json 工件全量对等（${commands.length} 条 web-ok 命令逐条）`, () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    tauriInvokeMock.mockReset();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('工件计数守卫（count == commands.length == 103）', () => {
    expect(artifact.count).toBe(commands.length);
    expect(commands.length).toBe(103);
  });

  for (const entry of commands) {
    describe(`命令 ${entry.name}`, () => {
      it('成功路径：双通道 resolve 深等同构 + 底层请求形状同构', async () => {
        const args = minimalArgs(entry);
        const fixture = responseFixture(entry.name);
        tauriInvokeMock.mockResolvedValue(fixture);
        fetchMock.mockResolvedValue(jsonResponse(200, fixture));

        const [tauriResult, httpResult] = await Promise.all([
          new TauriTransport().invoke(entry.name, args),
          new HttpTransport().invoke(entry.name, args),
        ]);

        expect(tauriResult).toEqual(httpResult);
        expect(httpResult).toEqual(fixture);
        // 底层请求形状同构：invoke(cmd, args) ⇄ fetch(POST /api/cmd/{cmd}, body=JSON.stringify(args ?? {}))
        expect(tauriInvokeMock).toHaveBeenCalledTimes(1);
        expect(tauriInvokeMock).toHaveBeenCalledWith(entry.name, args);
        expect(fetchMock).toHaveBeenCalledTimes(1);
        const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
        expect(url).toBe(`/api/cmd/${entry.name}`);
        expect(init.method).toBe('POST');
        expect(init.credentials).toBe('same-origin');
        expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
        expect(init.body).toBe(JSON.stringify(args ?? {}));
      });

      it('AppError 路径：双通道 reject 深等同构（200+判别头 ⇄ invoke rejection）', async () => {
        const args = minimalArgs(entry);
        const appError = { ValidationError: '参数反序列化失败: fixture' };
        tauriInvokeMock.mockRejectedValue(appError);
        fetchMock.mockResolvedValue(jsonResponse(200, appError, { 'x-egosync-app-error': '1' }));

        await expect(new TauriTransport().invoke(entry.name, args)).rejects.toEqual(appError);
        await expect(new HttpTransport().invoke(entry.name, args)).rejects.toEqual(appError);
        expect(tauriInvokeMock).toHaveBeenCalledWith(entry.name, args);
        const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
        expect(url).toBe(`/api/cmd/${entry.name}`);
        expect(init.body).toBe(JSON.stringify(args ?? {}));
      });
    });
  }
});

describe(`AppError ${variants.length} variant 黄金用例（error.rs 源码机械枚举）`, () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    tauriInvokeMock.mockReset();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('variant 清单计数钉（error.rs 现役声明 13 个）', () => {
    expect(variants).toHaveLength(13);
  });

  for (const variant of variants) {
    it(`${variant}：双通道 reject 同一单键 map`, async () => {
      // 工件驱动命令名（零手写）
      const command = commands[0]!.name;
      const appError = { [variant]: '示例错误消息' };
      tauriInvokeMock.mockRejectedValue(appError);
      fetchMock.mockResolvedValue(jsonResponse(200, appError, { 'x-egosync-app-error': '1' }));

      await expect(new TauriTransport().invoke(command)).rejects.toEqual(appError);
      await expect(new HttpTransport().invoke(command)).rejects.toEqual(appError);
    });
  }

  it('判别头缺席时单键 variant 名形状的成功响应不被误判（判别头的存在理由）', async () => {
    const command = commands[0]!.name;
    // 成功响应恰为「单键 variant 名」对象——按形状探测必误判，判别头是唯一信号
    const successBody = { [variants[0]!]: 'success-looking payload' };
    tauriInvokeMock.mockResolvedValue(successBody);
    fetchMock.mockResolvedValue(jsonResponse(200, successBody));

    const [tauriResult, httpResult] = await Promise.all([
      new TauriTransport().invoke(command),
      new HttpTransport().invoke(command),
    ]);
    expect(tauriResult).toEqual(httpResult);
    expect(httpResult).toEqual(successBody);
  });
});

describe('传输层错误形状冻结（仅 HTTP 侧存在；Tauri 分支无此路径）', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it.each([
    [401, { error: 'unauthorized' }],
    [429, { error: 'rate limit exceeded' }],
    [404, { error: 'unknown command' }],
  ] as const)('HTTP %s ⇒ reject HttpTransportError{status, body}（形状冻结）', async (status, body) => {
    fetchMock.mockResolvedValue(jsonResponse(status, body));
    const command = commands[0]!.name;
    const err = await new HttpTransport().invoke(command).then(
      () => {
        throw new Error('应 reject');
      },
      (e: unknown) => e
    );
    expect(err).toBeInstanceOf(HttpTransportError);
    expect((err as HttpTransportError).status).toBe(status);
    expect((err as HttpTransportError).body).toEqual(body);
    // 401 不触发任何跳转（全局拦截归 16.1）——transport 层只 reject
  });

  it('网络失败（fetch reject）⇒ HttpTransportError{status: 0, body: null}', async () => {
    fetchMock.mockRejectedValue(new TypeError('Failed to fetch'));
    const command = commands[0]!.name;
    const err = await new HttpTransport().invoke(command).then(
      () => {
        throw new Error('应 reject');
      },
      (e: unknown) => e
    );
    expect(err).toBeInstanceOf(HttpTransportError);
    expect((err as HttpTransportError).status).toBe(0);
    expect((err as HttpTransportError).body).toBeNull();
  });
});
