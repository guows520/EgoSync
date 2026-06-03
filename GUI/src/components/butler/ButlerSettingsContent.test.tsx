import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ButlerSettingsContent } from './ButlerSettingsContent';
import { appService } from '../../services/appService';

vi.mock('../../services/appService', () => ({
  appService: {
    getButlerSkills: vi.fn(),
    updateButlerSkills: vi.fn(),
  },
}));

describe('ButlerSettingsContent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
    });
  });

  it('展示并持久化管家 Skill 配置', async () => {
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: true,
    });

    render(<ButlerSettingsContent />);

    expect(await screen.findByText('Skill 配置')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'find-skills' })).toBeChecked();
    expect(screen.getByRole('switch', { name: 'skill-creator' })).not.toBeChecked();
    expect(screen.queryByText('保存中...')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('switch', { name: 'skill-creator' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: true,
      });
    });
    expect(await screen.findByText('Skill 配置已保存')).toBeInTheDocument();
    expect(screen.queryByText('保存中...')).not.toBeInTheDocument();
  });
});
