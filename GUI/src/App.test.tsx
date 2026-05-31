import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import App from './App'
import { appService } from './services/appService'
import { roleService } from './services/roleService'
import { useTauriEvent } from './hooks/useTauriEvent'
import type { Role } from './types/role'

vi.mock('./services/appService', () => ({
  appService: {
    isFirstLaunch: vi.fn(),
    completeOnboarding: vi.fn(),
    isLlmConfigured: vi.fn(),
  },
}))

vi.mock('./services/roleService', () => ({
  roleService: {
    create: vi.fn(),
    list: vi.fn(),
    listArchived: vi.fn(),
    update: vi.fn(),
    archive: vi.fn(),
    restore: vi.fn(),
    delete: vi.fn(),
  },
}))

vi.mock('./hooks/useTauriEvent', () => ({
  useTauriEvent: vi.fn(),
}))

vi.mock('./components/layout/Sidebar', () => ({
  Sidebar: () => <nav aria-label="sidebar" />,
}))

vi.mock('./components/butler/ButlerView', () => ({
  ButlerView: () => <div>管家视图</div>,
}))

vi.mock('./components/role/RoleView', () => ({
  RoleView: () => <div>角色视图</div>,
}))

vi.mock('./components/onboarding/OnboardingView', () => ({
  OnboardingView: () => <div>引导视图</div>,
}))

vi.mock('./components/settings/GlobalSettingsModal', () => ({
  GlobalSettingsModal: () => <div>设置</div>,
}))

vi.mock('./components/modals/ArbitrationModal', () => ({
  ArbitrationModal: () => <div>仲裁</div>,
}))

vi.mock('./components/modals/WeeklyReviewModal', () => ({
  WeeklyReviewModal: () => <div>复盘</div>,
}))

vi.mock('./components/modals/TaskModal', () => ({
  TaskModal: () => <div>任务</div>,
}))

vi.mock('./components/modals/AddRoleModal', () => ({
  AddRoleModal: () => <div>添加角色</div>,
}))

vi.mock('./components/notifications/NotificationPanel', () => ({
  NotificationPanel: () => <div>通知</div>,
}))

const createdRole: Role = {
  id: 'role-fitness',
  name: '健身教练',
  icon: 'dumbbell',
  color: '#10B981',
  goal: '保持稳定训练',
  personalityPrompt: '',
  status: 'active',
  energy: 100,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-05-30T00:00:00Z',
  updatedAt: '2026-05-30T00:00:00Z',
}

interface RoleProposedPayload {
  conversationId: string
  name: string
  icon: string | null
  color: string | null
  goal: string | null
}

let roleProposedHandler: ((payload: RoleProposedPayload) => void) | undefined

function mockNormalLaunch() {
  vi.mocked(appService.isFirstLaunch).mockResolvedValue(false)
  vi.mocked(roleService.list).mockResolvedValue([])
  vi.mocked(roleService.listArchived).mockResolvedValue([])
  vi.mocked(roleService.create).mockResolvedValue(createdRole)
  vi.mocked(useTauriEvent).mockImplementation((eventName, handler) => {
    if (eventName === 'role:proposed') {
      roleProposedHandler = handler as (payload: RoleProposedPayload) => void
    }
  })
}

describe('App', () => {
  beforeEach(() => {
    roleProposedHandler = undefined
    vi.clearAllMocks()
    mockNormalLaunch()
  })

  it('renders without crashing', () => {
    const { container } = render(<App />)
    expect(container).toBeTruthy()
  })

  it('opens the butler role proposal modal and refreshes roles after confirmation', async () => {
    render(<App />)

    await waitFor(() => expect(roleService.list).toHaveBeenCalledTimes(1))
    expect(roleProposedHandler).toBeDefined()

    roleProposedHandler?.({
      conversationId: 'conv-butler',
      name: '健身教练',
      icon: 'dumbbell',
      color: '#10B981',
      goal: '保持稳定训练',
    })

    expect(await screen.findByRole('dialog')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '创建' }))

    await waitFor(() => {
      expect(roleService.create).toHaveBeenCalledWith({
        name: '健身教练',
        icon: 'dumbbell',
        color: '#10B981',
        goal: '保持稳定训练',
      })
    })
    await waitFor(() => expect(roleService.list).toHaveBeenCalledTimes(2))
  })
})
