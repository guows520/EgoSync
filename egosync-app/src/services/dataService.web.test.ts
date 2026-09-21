// Story 17.3 评审补丁（验证缺口层 V3）：云端备份端点的 fetch 客户端
// （unwrapCloudResponse / webExport / webImport）零直接测试——设置页
// 测试把整个 dataService mock 掉，真实 fetch/解包代码从未被执行。
// 本文件以 http.test.ts 的 fetch stub 范式直测：成功面、AppError
// 判别头单键 map、非 200 白名单、401 全局会话失效信号、非 JSON 成功体、
// 体积预检（fetch 前拒绝）。逻辑契约见 transport/contract.test.ts
//（判别头常量三副本 source-scan 门禁）。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { subscribeFrontendLocalEvent } from '@/transport/localEvents';
import { dataService } from './dataService';

/** http.test.ts 同款 Response stub（headers.get 大小写不敏感）。 */
function stubResponse(
  status: number,
  body: unknown,
  headers: Record<string, string> = {}
): Response {
  const text = typeof body === 'string' ? body : JSON.stringify(body);
  const lower = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    status,
    ok: status >= 200 && status < 300,
    headers: { get: (name: string) => lower.get(name.toLowerCase()) ?? null },
    text: () => Promise.resolve(text),
    blob: () => Promise.resolve(new Blob([text], { type: 'application/json' })),
  } as unknown as Response;
}

describe('dataService 云端备份端点客户端（/api/export、/api/import）', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('webImport 成功：200 JSON 响应解包为 WebImportResult', async () => {
    fetchMock.mockResolvedValue(
      stubResponse(200, {
        imported: { rolesCount: 1, tasksCount: 1, memoriesCount: 1, conversationsCount: 1, messagesCount: 1 },
        missingSecrets: [],
      })
    );
    const result = await dataService.webImport(new File(['{}'], 'p.json'));
    expect(result.imported.rolesCount).toBe(1);
    expect(result.missingSecrets).toEqual([]);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0]!;
    expect(url).toBe('/api/import');
    expect(init.method).toBe('POST');
    expect(init.credentials).toBe('same-origin');
  });

  it('webImport AppError：200 + 判别头 ⇒ reject 单键 map（与 invoke 错误通道同构）', async () => {
    fetchMock.mockResolvedValue(
      stubResponse(200, { ValidationError: '导出包格式不正确' }, { 'x-egosync-app-error': '1' })
    );
    await expect(
      dataService.webImport(new File(['{}'], 'p.json'))
    ).rejects.toEqual({ ValidationError: '导出包格式不正确' });
  });

  it('webImport 401：reject HTTP 文案，且发射全局 auth:unauthorized（AuthGate 回登录面）', async () => {
    fetchMock.mockResolvedValue(stubResponse(401, { error: 'unauthorized' }));
    const events: string[] = [];
    const unlisten = subscribeFrontendLocalEvent('auth:unauthorized', () => events.push('auth:unauthorized'));
    await expect(
      dataService.webImport(new File(['{}'], 'p.json'))
    ).rejects.toThrow('HTTP 401');
    expect(events).toEqual(['auth:unauthorized']);
    unlisten();
  });

  it('webImport 非 JSON 成功体（反代故障页）：reject 解析失败错误，不返回 HTML 当结果', async () => {
    fetchMock.mockResolvedValue(stubResponse(200, '<html>502 Bad Gateway</html>'));
    await expect(
      dataService.webImport(new File(['{}'], 'p.json'))
    ).rejects.toThrow('不是有效的 JSON');
  });

  it('webImport 体积预检：>50MB 包在 fetch 前拒绝（免整包上传后 413）', async () => {
    const oversized = new File(['x'], 'big.json');
    Object.defineProperty(oversized, 'size', { value: 50 * 1024 * 1024 + 1 });
    await expect(dataService.webImport(oversized)).rejects.toThrow('50MB');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('webExport 成功：200 无判别头 ⇒ Blob', async () => {
    fetchMock.mockResolvedValue(stubResponse(200, { exportVersion: '1.0' }));
    const blob = await dataService.webExport();
    expect(blob).toBeInstanceOf(Blob);
    const [url, init] = fetchMock.mock.calls[0]!;
    expect(url).toBe('/api/export');
    expect(init.method).toBe('GET');
    expect(init.credentials).toBe('same-origin');
  });

  it('webExport AppError：200 + 判别头 + 非 JSON 错误体 ⇒ reject 明确错误（不抛字符串）', async () => {
    fetchMock.mockResolvedValue(
      stubResponse(200, 'gateway junk', { 'x-egosync-app-error': '1' })
    );
    await expect(dataService.webExport()).rejects.toThrow('无法解析的错误响应');
  });
});
