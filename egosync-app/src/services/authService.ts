// Story 16.1：认证面前端调用——REST 直连（仿 http.ts getAuthStatus 范式），
// 不进 invoke 命令通道（/api/cmd/* 是认证后业务面；认证面是 /api/auth/*、
// /api/setup 的独立 REST 契约）。
//
// - status 消费走 transport 层的 getAuthStatus（带限流预算保护的缓存通道，
//   AuthGate 直连——缓存失效由登录/setup 成功与登出路径触发）；
// - login/setup/logout：POST 直连 + credentials:'same-origin'（Cookie
//   自动携带/清除）；
// - 错误形状：非 200 ⇒ reject HttpTransportError{status, body}（401/404/
//   429），网络失败 ⇒ HttpTransportError(0, null)——与 invoke 通道同构，
//   视图层按 status 分流文案。

import { HttpTransportError } from '@/transport';

export const authService = {
  /** `POST /api/auth/login {token}` → 200 + Set-Cookie（30 天持久）。 */
  login: (token: string): Promise<void> => postToken('/api/auth/login', token),

  /** `POST /api/setup {token}` → 200（首访初始化；404 = env 锁定/已初始化）。 */
  setup: (token: string): Promise<void> => postToken('/api/setup', token),

  /**
   * `POST /api/auth/logout` → 幂等 200 + Cookie 过期（服务端删会话行）。
   * 网络失败不阻止本地登出流程（调用方 catch 后继续回登录页）。
   */
  logout: (): Promise<void> => postJson('/api/auth/logout', {}),
};

/** POST JSON（token 形态：body `{token}`）。 */
async function postToken(path: string, token: string): Promise<void> {
  await postJson(path, { token });
}

/** POST JSON + 同源 Cookie；非 200 ⇒ HttpTransportError；网络失败 ⇒ status 0。 */
async function postJson(path: string, body: Record<string, unknown>): Promise<void> {
  const res = await fetch(path, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  }).catch(() => {
    // 网络失败（fetch reject）：与 invoke 通道同形状（status 0 + body null）
    throw new HttpTransportError(0, null);
  });
  // text() 的 onRejected 只捕「响应体读取中断」（与 invoke 评审 G3 同款）
  await res.text().then(
    text => {
      if (res.status !== 200) {
        throw new HttpTransportError(res.status, parseJsonOrText(text));
      }
    },
    () => {
      throw new HttpTransportError(0, null);
    }
  );
}

/** body 解析：JSON 可解析则对象/标量，否则原始文本。 */
function parseJsonOrText(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}
