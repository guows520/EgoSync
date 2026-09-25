// EgoSync Service Worker（Story 16.4）——仅应用外壳缓存，业务数据零落盘。
//
// 缓存纪律（NFR-C3 红线，与 16.1 index.html no-cache 启动协商对齐）：
// - install：只预缓存「外壳清单」（index/manifest/favicon/图标）——hash 资产
//   每次构建文件名都变，install 清单无法预知，改为运行时 runtime caching；
// - hash 静态资产（/assets/* 等）：cache-first，命中即用、未命中回源并补缓存
//   （内容寻址 ⇒ 无失效问题；发版后旧文件仍可被旧 index 引用，不 404 白屏）；
// - index.html（导航请求）：stale-while-revalidate——先给缓存副本保证秒开，
//   同时回源校验并更新缓存（服务端 no-cache 语义不变：每次导航都回源）；
// - /api/* 与 SSE：零缓存。fetch/XHR 直通网络；EventSource 流本就不经过 SW
//   fetch 事件（浏览器不拦截），天然零缓存；
// - 刷新 = 服务端重取语义不变：业务数据一律不落 localStorage/IndexedDB/SW。
//
// 注册入口唯一：src/main.tsx（仅生产构建 + 浏览器宿主）；注册失败（隐私模式
// 等）静默降级——应用功能全可用，只是无离线外壳。

const CACHE_NAME = 'egosync-shell-v1';

// 外壳预缓存清单（不含 hash 资产——见文件头注释）。
const SHELL_URLS = [
  '/',
  '/index.html',
  '/manifest.webmanifest',
  '/favicon.svg',
  '/favicon.ico',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/icons/apple-touch-icon.png',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      // 逐个缓存（非 addAll 原子语义）：定制部署缺某个外壳文件（如自有
      // favicon）不拖垮整个外壳缓存，缺什么跳什么。
      .then((cache) => Promise.all(
        SHELL_URLS.map((u) => cache.add(u).catch((err) => console.warn('[sw] 跳过缺失外壳资源:', u, err))),
      ))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)),
      ))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // 只接管同源 GET；跨源与非 GET 一律直通（不缓存、不干预）。
  if (request.method !== 'GET' || url.origin !== self.location.origin) return;

  // 业务 API：network-only 零缓存（含 /api/cmd 与 /api/events 的 fetch 形态）。
  if (url.pathname === '/api' || url.pathname.startsWith('/api/')) return;

  // 导航请求（地址栏进入/刷新/SPA 深链）：index.html stale-while-revalidate。
  if (request.mode === 'navigate') {
    event.respondWith((async () => {
      const cache = await caches.open(CACHE_NAME);
      const cached = await cache.match('/index.html');
      const network = fetch(request)
        .then((res) => {
          if (res && res.ok) cache.put('/index.html', res.clone());
          return res;
        })
        .catch(() => null);
      // 先给缓存副本（秒开），后台回源更新；无缓存副本才等网络。
      return cached ?? network ?? Response.error();
    })());
    return;
  }

  // 其余同源 GET 静态资产（hash 资产/favicon/图标/manifest）：cache-first。
  event.respondWith((async () => {
    const cache = await caches.open(CACHE_NAME);
    const cached = await cache.match(request);
    if (cached) return cached;
    const res = await fetch(request);
    if (res && res.ok && res.type === 'basic') cache.put(request, res.clone());
    return res;
  })());
});
