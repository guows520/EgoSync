export type TaskQuadrant = 'Q1' | 'Q2' | 'Q3' | 'Q4';
export type TaskProtectionStatus = 'normal' | 'at_risk';

/** Story 3.3: 置信度低于此阈值的任务在 UI 中显示「不确定」标记。
 *  与后端 `services::task_classifier::CONFIDENCE_UNCERTAINTY_THRESHOLD` 保持同步。 */
export const CLASSIFICATION_UNCERTAINTY_THRESHOLD = 0.8;

export type TaskOwnerType = 'role' | 'butler';

export interface Task {
  id: string;
  ownerType: TaskOwnerType;
  roleId: string | null;
  title: string;
  deadline: string | null;
  quadrant: TaskQuadrant;
  isBigRock: boolean;
  isCompleted: boolean;
  completedAt: string | null;
  sortOrder: number;
  protectionStatus: TaskProtectionStatus;
  /** 自动分类置信度，0..1。null 表示尚未自动分类（如手动覆盖路径）。 */
  confidence: number | null;
  /** Story 3.3：true 表示用户在 TaskModal 中显式选择了 quadrant，
   *  自动分类与临期升 Q1 都会跳过该任务。 */
  manualOverride: boolean;
  /** Story 3.3：最近一次 quadrant 变更原因（中文短句），用于在卡片上展示「为什么放在这里」。 */
  classificationReason: string | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface CreateTaskInput {
  ownerType?: TaskOwnerType;
  roleId?: string | null;
  title: string;
  deadline?: string;
  /** Story 3.3：未提供时由后端自动分类，提供时立即标记为手动覆盖。 */
  quadrant?: TaskQuadrant;
  isBigRock?: boolean;
}

export interface UpdateTaskInput {
  title?: string;
  deadline?: string | null;
  /** Story 3.3：用户显式修改时，后端会将 manualOverride 置为 true 并阻断后续自动分类。 */
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

/** 判断任务是否应在 UI 中显示「不确定」标记：
 *  仅自动分类（manualOverride = false）且 confidence 低于阈值时返回 true。 */
export function isClassificationUncertain(task: Task): boolean {
  if (task.manualOverride) return false;
  if (task.confidence === null || task.confidence === undefined) return false;
  return task.confidence < CLASSIFICATION_UNCERTAINTY_THRESHOLD;
}
