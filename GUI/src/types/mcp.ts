export type McpServerType = 'sse' | 'streamable_http' | 'stdio' | 'http_sse' | 'command';

export interface McpServer {
  id: string;
  name: string;
  serverType: McpServerType;
  commandOrUrl: string;
  envRefs: string;
  description: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateMcpServerInput {
  name: string;
  serverType: McpServerType;
  commandOrUrl: string;
  envRefs?: string;
  description?: string;
  enabled: boolean;
}

export interface UpdateMcpServerInput {
  name?: string;
  serverType?: McpServerType;
  commandOrUrl?: string;
  envRefs?: string;
  description?: string;
  enabled?: boolean;
}
