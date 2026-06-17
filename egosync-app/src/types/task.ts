export type TaskQuadrant = 'Q1' | 'Q2' | 'Q3' | 'Q4';
export type TaskProtectionStatus = 'normal' | 'at_risk';

export interface Task {
  id: string;
  roleId: string;
  title: string;
  deadline: string | null;
  quadrant: TaskQuadrant;
  isBigRock: boolean;
  isCompleted: boolean;
  completedAt: string | null;
  sortOrder: number;
  protectionStatus: TaskProtectionStatus;
  confidence: number | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface CreateTaskInput {
  roleId: string;
  title: string;
  deadline?: string;
  quadrant?: TaskQuadrant;
  isBigRock?: boolean;
}

export interface UpdateTaskInput {
  title?: string;
  deadline?: string | null;
  quadrant?: TaskQuadrant;
  isBigRock?: boolean;
}

export interface TaskActions {
  createTask: (input: CreateTaskInput) => Promise<void>;
  updateTask: (id: string, input: UpdateTaskInput) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;
  reorderTasks: (taskIds: string[]) => Promise<void>;
  toggleComplete: (id: string, isCompleted: boolean) => Promise<void>;
}
