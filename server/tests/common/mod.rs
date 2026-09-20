//! 进程内 server 集成测试基建（relay-server InProcessServer 先例）。
//!
//! [`InProcessServer`]：真实 TCP 监听 + axum serve（ConnectInfo 供限流按
//! 直连 IP 计数），状态由调用方注入（生产路由 [`build_router`] 复用——
//! 测试面即生产面）。**serve 栈与 main.rs 生产栈同款**：IdleTimeoutListener
//! + RemoteAddr 连接信息（120s 空闲阈值在测试时限内不触发——测试不感知，
//! 但中间件/提取器签名与生产完全一致）。

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use egosync_server::AppState;
use egosync_server::idle_timeout::{IdleTimeoutListener, RemoteAddr, IDLE_TIMEOUT_SECS};
use tokio::net::TcpListener;

/// 进程内 server：随机端口 + 全量路由。
pub struct InProcessServer {
    pub state: Arc<AppState>,
    pub addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl InProcessServer {
    /// 以给定状态拉起 server（随机端口；API-only——无静态面）。
    pub async fn start(state: Arc<AppState>) -> Self {
        Self::start_with_static(state, None).await
    }

    /// 以给定状态与静态目录拉起 server（随机端口；`Some` ⇒ 静态面挂载）。
    ///
    /// 16.1 静态测试经 fixture dist 目录注入；默认 [`Self::start`] 不挂
    /// 静态面（不读工作区真实 `../egosync-app/dist`——测试面与构建产物解耦）。
    pub async fn start_with_static(
        state: Arc<AppState>,
        static_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let app = egosync_server::build_router(state.clone(), static_dir);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        // 生产同款：空闲超时监听器（120s 阈值远超测试时长——不干扰）
        let listener = IdleTimeoutListener::new(listener, Duration::from_secs(IDLE_TIMEOUT_SECS));
        let handle = tokio::spawn(async move {
            axum::serve(listener, app.into_make_service_with_connect_info::<RemoteAddr>())
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

/// HTTP 客户端便捷封装（会话经参数显式传递——精确断言 Cookie 形状）。
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::builder()
                .build()
                .expect("build test client"),
        }
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

    /// GET（带可选 Bearer 令牌与可选 Origin——Story 16.3 桌面远程通道）。
    pub async fn get_bearer(
        &self,
        url: &str,
        bearer: Option<&str>,
        origin: Option<&str>,
    ) -> reqwest::Response {
        let mut req = self.inner.get(url);
        if let Some(bearer) = bearer {
            req = req.bearer_auth(bearer);
        }
        if let Some(origin) = origin {
            req = req.header(reqwest::header::ORIGIN, origin);
        }
        req.send().await.expect("GET 请求（Bearer）")
    }

    /// OPTIONS（CORS preflight 探测——Story 16.3）。
    pub async fn options(&self, url: &str, origin: Option<&str>) -> reqwest::Response {
        let mut req = self.inner.request(reqwest::Method::OPTIONS, url);
        if let Some(origin) = origin {
            req = req.header(reqwest::header::ORIGIN, origin);
            // 预检必带 Access-Control-Request-Method（浏览器形态）
            req = req.header("Access-Control-Request-Method", "POST");
        }
        req.send().await.expect("OPTIONS 请求")
    }

    /// POST JSON（带可选 Bearer 令牌与可选 Origin——Story 16.3）。
    pub async fn post_json_bearer<T: serde::Serialize>(
        &self,
        url: &str,
        body: Option<&T>,
        bearer: Option<&str>,
        origin: Option<&str>,
    ) -> reqwest::Response {
        let mut req = self.inner.post(url);
        if let Some(body) = body {
            req = req.json(body);
        }
        if let Some(bearer) = bearer {
            req = req.bearer_auth(bearer);
        }
        if let Some(origin) = origin {
            req = req.header(reqwest::header::ORIGIN, origin);
        }
        req.send().await.expect("POST 请求（Bearer）")
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
