import { invoke } from '@tauri-apps/api/core';
import type { Role, CreateRoleInput, UpdateRoleInput, UpdateRoleProactivityInput, UpdateRoleSkillsInput } from '../types/role';

export const roleService = {
  create: (input: CreateRoleInput) => invoke<Role>('role_create', { input }),
  list: () => invoke<Role[]>('role_list'),
  listArchived: () => invoke<Role[]>('role_list_archived'),
  update: (id: string, input: UpdateRoleInput) => invoke<Role>('role_update', { id, input }),
  updateSkills: (id: string, input: UpdateRoleSkillsInput) => invoke<Role>('role_update_skills', { id, input }),
  updateProactivity: (id: string, input: UpdateRoleProactivityInput) => invoke<Role>('role_update_proactivity', { id, input }),
  archive: (id: string) => invoke<Role>('role_archive', { id }),
  restore: (id: string) => invoke<Role>('role_restore', { id }),
  delete: (id: string) => invoke<void>('role_delete', { id }),
};