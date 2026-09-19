import '@testing-library/jest-dom'

// Story 15.5：全局 `__TAURI_INTERNALS__` 桩——测试环境默认 Tauri 分支的
// 机制基础（isTauriHost() 据此探测，getTransport() 懒单例选中
// TauriTransport，invoke/listen 行为与迁移前直连 @tauri-apps/api 一致）。
// 需要浏览器分支的测试（src/transport/*）自行构造 HttpTransport 并经
// vi.stubGlobal 注入假 fetch/EventSource，不依赖本桩。
;(window as any).__TAURI_INTERNALS__ = {
  invoke: (cmd: string) => {
    if (cmd === 'chat_get_butler_conversation') {
      return Promise.resolve({ id: 'mock-conv', roleId: null, startedAt: '', updatedAt: '' });
    }
    if (cmd === 'chat_get_history') {
      return Promise.resolve([]);
    }
    if (cmd === 'notification_list') {
      return Promise.resolve([]);
    }
    if (cmd === 'notification_count_unread') {
      return Promise.resolve(0);
    }
    return Promise.resolve(null);
  },
  transformCallback: (cb: any) => {
    const id = Math.random();
    (window as any)[`_${id}`] = cb;
    return id;
  },
  convertFileSrc: (src: string) => src,
  metadata: { currentWebview: { label: 'main' }, currentWindow: { label: 'main' } },
  plugins: {
    event: {
      registerListener: () => Promise.resolve(0),
      unregisterListener: () => Promise.resolve(),
    },
  },
}

const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
    get length() { return Object.keys(store).length; },
    key: (index: number) => Object.keys(store)[index] ?? null,
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})
