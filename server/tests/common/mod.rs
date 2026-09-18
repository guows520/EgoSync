//! 进程内 server 集成测试基建（relay-server InProcessServer 先例）。
//!
//! [`InProcessServer`]：真实 TCP 监听 + axum serve（ConnectInfo 供限流按
//! 直连 IP 计数），状态由调用方注入（生产路由 [`build_router`] 复用——
//! 测试面即生产面）。

use std::net::SocketAddr;
use std::sync::Arc;

use egosync_server::AppState;
use tokio::net::TcpListener;

/// 进程内 server：随机端口 + 全量路由。
pub struct InProcessServer {
    pub state: Arc<AppState>,
    pub addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl InProcessServer {
    /// 以给定状态拉起 server（随机端口）。
    pub async fn start(state: Arc<AppState>) -> Self {
        let app = egosync_server::build_router(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .expect("test server serve");
        });
        Self { state, addr, handle }
    }

    /// 完整 URL。
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

impl Drop for InProcessServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// HTTP 客户端便捷封装（手动 Cookie 管理——精确断言 Cookie 形状）。
pub struct Client {
    inner: reqwest::Client,
    cookie: Option<String>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::builder()
                .build()
                .expect("build test client"),
            cookie: None,
        }
    }

    /// 记住 Set-Cookie 中的会话值（裸 token，供 Cookie 头发送）。
    pub fn capture_session_cookie(&mut self, res: &reqwest::Response) -> Option<String> {
        let set_cookie = res.headers().get(reqwest::header::SET_COOKIE)?.to_str().ok()?;
        let value = set_cookie
            .split(';')
            .next()?
            .trim()
            .to_string();
        self.cookie = Some(value.clone());
        Some(value)
    }

    /// 已捕获的会话 Cookie 头值（如 `egosync_session=...`）。
    pub fn session(&self) -> Option<&str> {
        self.cookie.as_deref()
    }

    /// GET（带可选会话 Cookie 与可选 Origin）。
    pub async fn get(
        &self,
        url: &str,
        session: Option<&str>,
        origin: Option<&str>,
    ) -> reqwest::Response {
        let mut req = self.inner.get(url);
        if let Some(session) = session {
            req = req.header(reqwest::header::COOKIE, session);
        }
        if let Some(origin) = origin {
            req = req.header(reqwest::header::ORIGIN, origin);
        }
        req.send().await.expect("GET 请求")
    }

    /// POST JSON（带可选会话 Cookie 与可选 Origin）。
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: Option<&T>,
        session: Option<&str>,
        origin: Option<&str>,
    ) -> reqwest::Response {
        let mut req = self.inner.post(url);
        if let Some(body) = body {
            req = req.json(body);
        }
        if let Some(session) = session {
            req = req.header(reqwest::header::COOKIE, session);
        }
        if let Some(origin) = origin {
            req = req.header(reqwest::header::ORIGIN, origin);
        }
        req.send().await.expect("POST 请求")
    }

    /// POST 原始字节（body 上限测试用）。
    pub async fn post_bytes(
        &self,
        url: &str,
        body: Vec<u8>,
        session: Option<&str>,
    ) -> reqwest::Response {
        let mut req = self
            .inner
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        if let Some(session) = session {
            req = req.header(reqwest::header::COOKIE, session);
        }
        req.body(body).send().await.expect("POST bytes 请求")
    }
}

/// 登录并返回会话 Cookie 头值（`egosync_session=...`）。
pub async fn login(client: &Client, url: &str, token: &str) -> Option<String> {
    let body = serde_json::json!({ "token": token });
    let res = client
        .post_json(&format!("{}/api/auth/login", url), Some(&body), None, None)
        .await;
    let set_cookie = res.headers().get(reqwest::header::SET_COOKIE)?.to_str().ok()?;
    Some(set_cookie.split(';').next()?.trim().to_string())
}
