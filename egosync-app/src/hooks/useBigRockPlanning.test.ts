import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useBigRockPlanning } from './useBigRockPlanning';
import type { Role } from '../types/role';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeRole(overrides: Partial<Role> = {}): Role {
  return {
    id: 'r1',
    name: '产品经理',
    icon: 'briefcase',
    color: '#4F46E5',
    goal: '做好产品',
    personalityPrompt: '',
    status: 'active',
    energy: 80,
    skillsConfig: '{}',
    proactivityLevel: 'moderate',
    archivedAt: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('useBigRockPlanning', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('初始状态：suggestions 为 null，未加载', () => {
    const { result } = renderHook(() => useBigRockPlanning([makeRole()]));
    expect(result.current.suggestions).toBeNull();
    expect(result.current.isLoadingSuggestions).toBe(false);
    expect(result.current.isSaving).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('loadSuggestions 成功加载建议', async () => {
    const suggestions = [
      { roleId: 'r1', roleName: '产品经理', suggestions: ['Q3路线图定稿', '竞品分析'] },
    ];
    mockInvoke.mockResolvedValue(suggestions);

    const { result } = renderHook(() => useBigRockPlanning([makeRole()]));

    act(() => {
      result.current.loadSuggestions();
    });

    await waitFor(() => {
      expect(result.current.suggestions).not.toBeNull();
    });

    expect(result.current.suggestions).toHaveLength(1);
    expect(result.current.suggestions![0].roleId).toBe('r1');
    expect(result.current.suggestions![0].suggestions).toHaveLength(2);
    expect(result.current.isLoadingSuggestions).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('loadSuggestions 失败时降级为空数组并设置 error', async () => {
    mockInvoke.mockRejectedValue(new Error('LLM timeout'));

    const { result } = renderHook(() => useBigRockPlanning([makeRole()]));

    act(() => {
      result.current.loadSuggestions();
    });

    await waitFor(() => {
      expect(result.current.suggestions).not.toBeNull();
    });

    expect(result.current.suggestions).toEqual([]);
    expect(result.current.error).not.toBeNull();
    expect(result.current.isLoadingSuggestions).toBe(false);
  });

  it('savePlan 成功时返回创建的 Task 列表', async () => {
    const tasks = [
      { id: 't1', ownerType: 'role', roleId: 'r1', title: 'Q3路线图', quadrant: 'Q2', isBigRock: true, isCompleted: false, completedAt: null, sortOrder: 0, protectionStatus: 'normal', confidence: null, manualOverride: false, classificationReason: null, deadline: null, createdAt: '2026-06-27T10:00:00Z', updatedAt: '2026-06-27T10:00:00Z', deletedAt: null },
    ];
    mockInvoke.mockResolvedValue(tasks);

    const { result } = renderHook(() => useBigRockPlanning([makeRole()]));

    let savedTasks: any;
    await act(async () => {
      savedTasks = await result.current.savePlan([{ roleId: 'r1', title: 'Q3路线图' }]);
    });

    expect(savedTasks).toHaveLength(1);
    expect(savedTasks[0].isBigRock).toBe(true);
    expect(result.current.isSaving).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('savePlan 失败时设置 error 并 re-throw', async () => {
    mockInvoke.mockRejectedValue(new Error('db error'));

    const { result } = renderHook(() => useBigRockPlanning([makeRole()]));

    await act(async () => {
      try {
        await result.current.savePlan([{ roleId: 'r1', title: 'test' }]);
      } catch (e) {
        // expected
      }
    });

    expect(result.current.error).toBe('db error');
    expect(result.current.isSaving).toBe(false);
  });
});
