import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { dataService } from './dataService';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('dataService.dataExport', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('调用 data_export 命令并传入 formats 参数', async () => {
    const mockResult = {
      files: ['/tmp/export.json'],
      sqlitePath: null,
      jsonPath: '/tmp/export.json',
      markdownPath: null,
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await dataService.dataExport(['json']);

    expect(mockInvoke).toHaveBeenCalledWith('data_export', { formats: ['json'] });
    expect(result).toEqual(mockResult);
  });

  it('支持多格式导出', async () => {
    const mockResult = {
      files: ['/tmp/a.db', '/tmp/a.json', '/tmp/a.md'],
      sqlitePath: '/tmp/a.db',
      jsonPath: '/tmp/a.json',
      markdownPath: '/tmp/a.md',
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await dataService.dataExport(['sqlite', 'json', 'markdown']);

    expect(mockInvoke).toHaveBeenCalledWith('data_export', {
      formats: ['sqlite', 'json', 'markdown'],
    });
    expect(result.files).toHaveLength(3);
  });

  it('命令失败时抛出错误', async () => {
    mockInvoke.mockRejectedValue(new Error('Export failed'));

    await expect(dataService.dataExport(['json'])).rejects.toThrow('Export failed');
  });
});
