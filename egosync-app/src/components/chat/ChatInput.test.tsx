import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatInput } from './ChatInput';
import type { SelectableSkill } from '../../types/skill';

const mockSkills: SelectableSkill[] = [
  { key: 'registry:skill-1', name: 'ppt-generation', description: '生成PPT', kind: 'registry', sourceType: 'opencode' },
  { key: 'registry:skill-2', name: 'pdf-to-markdown', description: 'PDF转MD', kind: 'registry', sourceType: 'opencode' },
  { key: 'registry:skill-3', name: 'code-review', description: '代码审查', kind: 'registry', sourceType: 'custom' },
];

describe('ChatInput focus indicator', () => {
  /// 焦点状态只保留一层与原边框等宽的高亮，避免灰色边框、ring 和全局 outline 叠加。
  it('uses a transparent focus border, a 1px ring, and suppresses the global focus-visible outline', () => {
    render(<ChatInput onSend={vi.fn()} />);

    const input = screen.getByRole('combobox');
    expect(input).toHaveClass('focus:border-transparent');
    expect(input).toHaveClass('focus:ring-1');
    expect(input).toHaveClass('focus-visible:!outline-none');
  });
});
describe('ChatInput role accent', () => {
  /// AC-1 / AC-3: 角色视图的发送入口必须使用当前角色色温变量。
  /// 否则 App 虽然更新了 CSS 变量，用户仍然看不到角色色温切换。
  it('useRoleAccent 为 true 时发送按钮使用 --role-accent', () => {
    render(<ChatInput onSend={vi.fn()} useRoleAccent />);

    const button = screen.getByRole('button');
    expect(button).toHaveStyle({ backgroundColor: 'var(--role-accent)' });
  });
});

describe('ChatInput @Skill 选择器', () => {
  /// AC-1: 输入 @ 时应弹出候选列表，仅包含当前 Agent 已添加且启用的 Skill
  it('输入 @ 时弹出候选列表', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '帮我做 @' } });
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    expect(screen.getAllByRole('option')).toHaveLength(3);
  });

  /// AC-1: 继续输入按名称前缀过滤候选
  it('继续输入按名称前缀过滤候选', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@p' } });
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(2);
    expect(options[0]).toHaveTextContent('ppt-generation');
    expect(options[1]).toHaveTextContent('pdf-to-markdown');
  });

  /// AC-2: 无可用 Skill 时显示空状态
  it('无可用 Skill 时显示空状态提示', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={[]} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    expect(screen.getByText('当前没有可用的 Skill')).toBeInTheDocument();
  });

  /// AC-2: 有 Skill 但无匹配时显示无匹配提示
  it('有 Skill 但无匹配时显示无匹配提示', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@xyz' } });
    expect(screen.getByText('没有匹配的 Skill')).toBeInTheDocument();
  });

  /// AC-2: Escape 关闭候选列表
  it('Escape 关闭候选列表', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  /// AC-2: Enter 确认选择当前焦点项
  it('Enter 确认选择当前焦点项', () => {
    const onSelectedSkillChange = vi.fn();
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} onSelectedSkillChange={onSelectedSkillChange} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '帮我做 @' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSelectedSkillChange).toHaveBeenCalledWith('registry:skill-1');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  /// AC-2: 已选 Skill 显示为 chip，可删除
  it('已选 Skill 显示为 chip 且可删除', () => {
    const onSelectedSkillChange = vi.fn();
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} onSelectedSkillChange={onSelectedSkillChange} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(screen.getByText('ppt-generation')).toBeInTheDocument();
    // 删除 chip
    const removeBtn = screen.getByLabelText('移除 Skill 选择');
    fireEvent.click(removeBtn);
    expect(onSelectedSkillChange).toHaveBeenCalledWith(null);
    expect(screen.queryByText('ppt-generation')).not.toBeInTheDocument();
  });

  /// 选中标签会增高外层容器；发送按钮必须只相对输入框定位，避免垂直错位。
  it('Skill chip 与发送按钮分属外层和输入框定位容器', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    const sendButton = screen.getByLabelText('发送');
    const chip = screen.getByText('ppt-generation').closest('span');
    expect(sendButton.parentElement).toContainElement(input);
    expect(sendButton.parentElement).not.toContainElement(chip);
  });

  /// AC-7: 发送后清空 selectedSkillKey
  it('发送后清空已选 Skill', async () => {
    const onSelectedSkillChange = vi.fn();
    const onSend = vi.fn();
    render(<ChatInput onSend={onSend} availableSkills={mockSkills} onSelectedSkillChange={onSelectedSkillChange} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    // 输入消息内容并发送
    fireEvent.change(input, { target: { value: '帮我生成PPT' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSend).toHaveBeenCalledWith('帮我生成PPT');
    await waitFor(() => expect(onSelectedSkillChange).toHaveBeenLastCalledWith(null));
  });

  /// AC-2: @Skill 标记不是 content 的组成部分
  it('发送时 content 不包含 @Skill 标记', () => {
    const onSend = vi.fn();
    render(<ChatInput onSend={onSend} availableSkills={mockSkills} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '帮我做 @p' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    // 选择 ppt-generation 后 @p 被移除
    fireEvent.change(input, { target: { value: '帮我生成季度汇报' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSend).toHaveBeenCalledWith('帮我生成季度汇报');
  });

  /// AC-2: 无 availableSkills 时输入 @ 显示空状态
  it('无 availableSkills 时输入 @ 显示空状态提示', () => {
    render(<ChatInput onSend={vi.fn()} availableSkills={[]} />);
    const input = screen.getByRole('combobox');
    fireEvent.change(input, { target: { value: '@' } });
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    expect(screen.getByText('当前没有可用的 Skill')).toBeInTheDocument();
  });
});
