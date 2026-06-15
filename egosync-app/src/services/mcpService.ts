import { invoke } from '@tauri-apps/api/core';
import type { CreateMcpServerInput, McpServer, UpdateMcpServerInput } from '../types/mcp';

export const mcpService = {
  list: () => invoke<McpServer[]>('mcp_server_list'),
  listForRole: (roleId: string) => invoke<McpServer[]>('mcp_server_list_for_role', { roleId }),
  listAvailableForRole: (roleId: string) => invoke<McpServer[]>('mcp_server_list_available_for_role', { roleId }),
  create: (input: CreateMcpServerInput) => invoke<McpServer>('mcp_server_create', { input }),
  update: (id: string, input: UpdateMcpServerInput) => invoke<McpServer>('mcp_server_update', { id, input }),
  delete: (id: string) => invoke<void>('mcp_server_delete', { id }),
  test: (id: string) => invoke<void>('mcp_server_test', { id }),
  addToRole: (roleId: string, serverId: string) => invoke<void>('mcp_server_add_to_role', { roleId, serverId }),
  removeFromRole: (roleId: string, serverId: string) => invoke<void>('mcp_server_remove_from_role', { roleId, serverId }),
};
