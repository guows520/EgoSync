import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatInput } from './ChatInput';
import type { SelectableSkill } from '../../types/skill';

const mockSkills: SelectableSkill[] = [
  { key: 'registry:skill-1', name: 'ppt-generation', description: '生成PPT', kind: 'registry', sourceType: 'opencode' },
  { key: 'registry:skill-2', name: 'pdf-to-markdown', description: 'PDF转MD', kind: 'registry', sourceType: 'opencode' },
];

describe('ChatInput 无障碍', () => {
  it('输入框应有 aria-label', () => {
    render(<ChatInput onSend={vi.fn()} />);
    const input = screen.getByRole('combobox');
    expect(input).toHaveAttribute('aria-label');
  });

  it('使用自定义 placeholder 时 aria-label 应反映 placeholder', () => {
    render(<ChatInput onSend={vi.fn()} placeholder="跟产品经理说点什么..." />);
    const input = screen.getByRole('combobox');
    expect(input).toHaveAttribute('aria-label', '跟产品经理说点什么...');
  });

  it('无 placeholder 时 aria-label 应使用默认值', () => {
    render(<ChatInput onSend={vi.fn()} />);
    const input = screen.getByRole('combobox');
    expect(input.getAttribute('aria-label')).toBeTruthy();
  });
});

describe('ChatInput @Skill 无障碍', () => {
  /// AC-2: 候选列表必须有正确的 ARIA 角色
  it('候选列表具有 role="listbox" 且选项具有 role="option"', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    const listbox = screen.getByRole('listbox');
    expect(listbox).toHaveAttribute('aria-label', '可用 Skill 列表');
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(2);
    expect(options[0]).toHaveAttribute('aria-selected', 'true');
  });

  /// AC-2: 输入框 aria-expanded 随候选列表显示/隐藏切换
  it('输入框 aria-expanded 随候选列表显示切换', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    expect(input).toHaveAttribute('aria-expanded', 'false');
    fireEvent.change(input, { target: { value: '@' } });
    expect(input).toHaveAttribute('aria-expanded', 'true');
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(input).toHaveAttribute('aria-expanded', 'false');
  });

  /// AC-2: aria-activedescendant 跟踪当前焦点项
  it('aria-activedescendant 跟踪当前焦点项', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    expect(input).toHaveAttribute('aria-activedescendant', 'skill-option-0');
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(input).toHaveAttribute('aria-activedescendant', 'skill-option-1');
    fireEvent.keyDown(input, { key: 'ArrowUp' });
    expect(input).toHaveAttribute('aria-activedescendant', 'skill-option-0');
  });
});
