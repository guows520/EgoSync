import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AddRoleModal } from './AddRoleModal';
import { roleService } from '../../services/roleService';
import type { Role } from '../../types/role';
vi.mock('../../services/roleService', () => ({ roleService: { create: vi.fn() } }));
const createdRole: Role = { id:'role-1',name:'旅行规划师',icon:'briefcase',color:'#4F46E5',goal:'规划年度旅行',personalityPrompt:'',status:'active',energy:100,skillsConfig:'{}',proactivityLevel:'moderate',archivedAt:null,createdAt:'2026-07-22T00:00:00Z',updatedAt:'2026-07-22T00:00:00Z' };
describe('AddRoleModal 目标字段', () => {
 beforeEach(() => { vi.clearAllMocks(); vi.mocked(roleService.create).mockResolvedValue(createdRole); });
 it('提交去除首尾空白后的目标', async () => { const onAdd=vi.fn(); render(<AddRoleModal onClose={vi.fn()} onAdd={onAdd}/>); fireEvent.change(screen.getByLabelText('角色名称'),{target:{value:'旅行规划师'}}); fireEvent.change(screen.getByLabelText('目标'),{target:{value:'  规划年度旅行  '}}); fireEvent.click(screen.getByRole('button',{name:'创建角色'})); await waitFor(()=>expect(roleService.create).toHaveBeenCalledWith(expect.objectContaining({name:'旅行规划师',goal:'规划年度旅行'}))); expect(onAdd).toHaveBeenCalledWith(createdRole); });
 it('空白目标按 undefined 提交', async () => { render(<AddRoleModal onClose={vi.fn()} onAdd={vi.fn()}/>); fireEvent.change(screen.getByLabelText('角色名称'),{target:{value:'旅行规划师'}}); fireEvent.change(screen.getByLabelText('目标'),{target:{value:'   '}}); fireEvent.click(screen.getByRole('button',{name:'创建角色'})); await waitFor(()=>expect(roleService.create).toHaveBeenCalledWith(expect.objectContaining({goal:undefined}))); });
});
