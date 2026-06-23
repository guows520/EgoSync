import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { RoleSidebarIcon } from './RoleSidebarIcon';

const mockRole = {
  id: 'test-role',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  energy: 85,
};

const mockRoleLowEnergy = {
  ...mockRole,
  id: 'low-energy',
  name: '学习者',
  energy: 30,
};

const mockRoleMidEnergy = {
  ...mockRole,
  id: 'mid-energy',
  name: '家庭',
  energy: 55,
};

describe('RoleSidebarIcon', () => {
  it('renders a line-icon SVG when icon id is in whitelist', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    // Lucide React 渲染为 svg.lucide.lucide-briefcase
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg?.getAttribute('class') ?? '').toContain('lucide');
  });

  it('falls back to default Target icon when icon id is unknown', () => {
    const role = { ...mockRole, icon: 'unknown-icon-xyz' };
    const { container } = render(
      <RoleSidebarIcon
        role={role}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    // 仍应渲染出 svg（Target 回退）
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('has breathe class when not active', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const button = container.querySelector('button');
    expect(button?.classList.contains('breathe')).toBe(true);
  });

  it('does NOT have breathe class when active', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={true}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const button = container.querySelector('button');
    expect(button?.classList.contains('breathe')).toBe(false);
  });

  it('aria-label includes role name and energy', () => {
    render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const button = screen.getByRole('button');
    expect(button).toHaveAttribute('aria-label', '产品经理 - 能量值 85%');
  });

  it('Enter key triggers onClick', () => {
    const onClick = vi.fn();
    render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={onClick}
        onContextMenu={vi.fn()}
      />
    );
    const button = screen.getByRole('button');
    fireEvent.keyDown(button, { key: 'Enter' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('Space key triggers onClick', () => {
    const onClick = vi.fn();
    render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={onClick}
        onContextMenu={vi.fn()}
      />
    );
    const button = screen.getByRole('button');
    fireEvent.keyDown(button, { key: ' ' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('shows green energy dot for high energy (>=80)', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const dot = container.querySelector('[aria-hidden="true"]');
    expect(dot).toHaveStyle({ backgroundColor: '#10B981' });
  });

  it('shows amber energy dot for mid energy (40-79)', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRoleMidEnergy}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const dot = container.querySelector('[aria-hidden="true"]');
    expect(dot).toHaveStyle({ backgroundColor: '#F59E0B' });
  });

  it('shows red energy dot for low energy (<40)', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRoleLowEnergy}
        isActive={false}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const dot = container.querySelector('[aria-hidden="true"]');
    expect(dot).toHaveStyle({ backgroundColor: '#EF4444' });
  });

  it('applies inline backgroundColor style when active with hex color', () => {
    const { container } = render(
      <RoleSidebarIcon
        role={mockRole}
        isActive={true}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const button = container.querySelector('button');
    expect(button).toHaveStyle({ backgroundColor: '#4F46E5' });
  });

  it('applies Tailwind class when active with Tailwind color format', () => {
    const role = { ...mockRole, color: 'bg-indigo-600' };
    const { container } = render(
      <RoleSidebarIcon
        role={role}
        isActive={true}
        onClick={vi.fn()}
        onContextMenu={vi.fn()}
      />
    );
    const button = container.querySelector('button');
    expect(button?.classList.contains('bg-indigo-600')).toBe(true);
  });
});