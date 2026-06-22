import '@testing-library/jest-dom'

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
