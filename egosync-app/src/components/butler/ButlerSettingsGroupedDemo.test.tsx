import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { ButlerSettingsGroupedDemo } from './ButlerSettingsGroupedDemo';

describe('ButlerSettingsGroupedDemo', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('支持单个分组折叠和展开全部分组', () => {
    render(<ButlerSettingsGroupedDemo />);

    expect(screen.getByText('如何称呼您')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /使命与价值观/ }));
    expect(screen.queryByText('编辑使命宣言')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '收起全部' }));
    expect(screen.queryByText('如何称呼您')).not.toBeInTheDocument();
    expect(screen.getByText('基础信息')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '展开全部' }));
    expect(screen.getByText('如何称呼您')).toBeInTheDocument();
    expect(screen.getAllByText('重新启用')).toHaveLength(3);
  });

  it('保留折叠状态到本地存储', () => {
    const { unmount } = render(<ButlerSettingsGroupedDemo />);
    fireEvent.click(screen.getByRole('button', { name: /角色管理/ }));
    unmount();

    render(<ButlerSettingsGroupedDemo />);
    expect(screen.getAllByText('重新启用')).toHaveLength(3);
  });
});

