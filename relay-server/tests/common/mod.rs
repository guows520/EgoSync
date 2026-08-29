//! 集成测试公共基建：进程内 server、子进程 server、原始 HTTP 探活、
//! 测试客户端（register + responder 侧 XX 握手）。
//!
//! 各测试二进制只引用本模块的子集，共享 helper 的 dead_code 属预期。
#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use companion_proto::crypto::{generate_static_keypair, HandshakeSession};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;

/// 原始 HTTP GET 响应（测试断言只需状态码）。
pub struct RawHttpResponse {
    pub status: u16,
}

/// 进程内 server：绑定随机端口后交给 `axum::serve`。
pub struct InProcessServer {
    pub addr: SocketAddr,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl InProcessServer {
    pub async fn start(router: axum::Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("绑定随机端口失败");
        let addr = listener.local_addr().expect("获取本地地址失败");
        let (shutdown, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .expect("进程内 server 错误");
        });
        Self {
            addr,
            shutdown: Some(shutdown),
        }
    }

    /// `GET /healthz` 原始 HTTP 探活。
    pub async fn healthz(&self) -> RawHttpResponse {
        raw_http_get(self.addr, "/healthz").await
    }

    pub fn relay_url(&self) -> String {
        format!("ws://{}/relay", self.addr)
    }
}

impl Drop for InProcessServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// 以子进程方式拉起 relay 二进制（真进程重启断言用）。
pub struct SubprocessServer {
    pub addr: SocketAddr,
    child: Option<Child>,
    /// 后台线程持续回收的 stderr（防子进程日志写满 OS 管道缓冲而停摆）。
    stderr_buf: Option<Arc<Mutex<String>>>,
    stderr_thread: Option<std::thread::JoinHandle<()>>,
}

impl SubprocessServer {
    pub fn start(port: u16) -> Self {
        Self::start_with(port, None, None)
    }

    /// cwd：工作目录（None = 继承，零磁盘写断言用临时目录）；
    /// rust_log：`RUST_LOG` 值（None = 默认 info，日志纪律断言用 debug）。
    pub fn start_with(port: u16, cwd: Option<&PathBuf>, rust_log: Option<&str>) -> Self {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_relay-server"));
        cmd.env("RELAY_HOST", "127.0.0.1")
            .env("RELAY_PORT", port.to_string());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        if let Some(v) = rust_log {
            cmd.env("RUST_LOG", v);
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("拉起 relay-server 子进程失败");

        // 专线程持续读取 stderr：RUST_LOG=debug 时日志量可超过 OS 管道缓冲
        // （Linux 默认 64KB），不消费会反压阻塞子进程的写侧使其停摆。
        let stderr = child.stderr.take();
        let stderr_buf = Arc::new(Mutex::new(String::new()));
        let buf_for_thread = stderr_buf.clone();
        let stderr_thread = std::thread::spawn(move || {
            use std::io::Read;
            let Some(mut stderr) = stderr else { return };
            let mut chunk = [0u8; 4096];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break, // EOF（子进程退出）
                    Ok(n) => buf_for_thread
                        .lock()
                        .expect("stderr 缓冲锁中毒")
                        .push_str(&String::from_utf8_lossy(&chunk[..n])),
                }
            }
        });

        let addr = format!("127.0.0.1:{port}")
            .parse()
            .expect("地址解析失败");
        Self {
            addr,
            child: Some(child),
            stderr_buf: Some(stderr_buf),
            stderr_thread: Some(stderr_thread),
        }
    }

    pub fn healthz(&self) -> RawHttpResponse {
        blocking_raw_http_get(self.addr, "/healthz")
    }

    pub fn relay_url(&self) -> String {
        format!("ws://{}/relay", self.addr)
    }

    /// 等待 /healthz 就绪（子进程启动有延迟）；子进程提前退出则快速失败
    /// （典型原因：free_port 的微秒级窗口内端口被抢占，绑定 panic 退出）。
    pub fn wait_ready(&mut self) {
        for _ in 0..100 {
            if self.healthz().status == 200 {
                return;
            }
            if let Some(child) = self.child.as_mut() {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    panic!("relay 子进程启动即退出（端口被抢占或启动失败）");
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("relay 子进程 10s 内未就绪");
    }

    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// 杀掉子进程并取回全部 stderr（读取线程在子进程退出（EOF）后自然结束）。
    pub fn take_stderr(&mut self) -> String {
        self.kill();
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
        self.stderr_buf
            .take()
            .map(|buf| buf.lock().expect("stderr 缓冲锁中毒").clone())
            .unwrap_or_default()
    }
}

impl Drop for SubprocessServer {
    fn drop(&mut self) {
        if self.child.is_some() {
            self.kill();
        }
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
    }
}

/// 随机空闲端口。
pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("绑定随机端口失败")
        .local_addr()
        .expect("获取本地地址失败")
        .port()
}

/// 生成一对 X25519 静态密钥（测试客户端身份）。
pub fn keypair() -> (Vec<u8>, Vec<u8>) {
    generate_static_keypair().expect("生成密钥对失败")
}

/// 小写 hex 编码（测试断言用）。
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 诚实测试客户端：连上 /relay 后以真实静态私钥完成 responder 侧 XX 握手。
pub struct RelayClient {
    ws: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
}

/// 一次 recv 的结果（带超时）。
pub enum Recv {
    Binary(Vec<u8>),
    Close,
}

impl RelayClient {
    /// 仅建立 WS 连接，不发送任何消息（驱动式测试用）。
    pub async fn connect(url: &str) -> Self {
        let (ws, _resp) = connect_async(url)
            .await
            .expect("WS 连接失败");
        Self { ws }
    }

    /// 连接并以真实静态私钥完成 responder 侧 XX 握手（诚实客户端）。
    pub async fn register_and_handshake(
        url: &str,
        relay_id: &str,
        role: &str,
        static_priv: &[u8],
    ) -> Self {
        let mut client = Self::connect(url).await;
        let register = format!(
            r#"{{"type":"register","relayId":"{relay_id}","role":"{role}"}}"#
        );
        client
            .ws
            .send(WsMessage::Text(register.into()))
            .await
            .expect("发送 register 失败");

        let mut session =
            HandshakeSession::responder(static_priv).expect("构建 responder 失败");
        // -> e
        let m1 = client.recv_binary().await;
        session.read_message(&m1).expect("读 m1 失败");
        // <- e,ee,s,es
        let m2 = session.write_message(&[]).expect("写 m2 失败");
        client.send_binary(&m2).await;
        // -> s,se
        let m3 = client.recv_binary().await;
        session.read_message(&m3).expect("读 m3 失败");
        client
    }

    pub async fn send_binary(&mut self, data: &[u8]) {
        self.ws
            .send(WsMessage::Binary(data.to_vec().into()))
            .await
            .expect("发送 binary 失败");
    }

    pub async fn send_text(&mut self, text: &str) {
        self.ws
            .send(WsMessage::Text(text.to_string().into()))
            .await
            .expect("发送 text 失败");
    }

    pub async fn send_raw_binary(&mut self, data: &[u8]) {
        self.ws
            .send(WsMessage::Binary(data.to_vec().into()))
            .await
            .expect("发送 raw binary 失败");
    }

    /// 接收一条 binary 消息（5s 超时；收到非 binary/close 直接 panic）。
    pub async fn recv_binary(&mut self) -> Vec<u8> {
        match self.recv_with_timeout().await {
            Recv::Binary(b) => b,
            Recv::Close => panic!("期望 binary，但连接被关闭"),
        }
    }

    /// 在 5s 内观测连接是否被对端关闭（true = 已关闭）。
    pub async fn is_closed(&mut self) -> bool {
        matches!(self.recv_with_timeout().await, Recv::Close)
    }

    /// 在给定预算内观测连接是否被对端关闭（true = 已关闭；预算耗尽未关 = false）。
    /// 用于区分「时限内关闭」与「更晚才关」的超时语义回归。
    pub async fn closed_within(&mut self, budget: Duration) -> bool {
        let fut = async {
            loop {
                match self.ws.next().await {
                    None => return true,
                    Some(Ok(WsMessage::Close(_))) => return true,
                    Some(Ok(_)) => continue, // Ping/Pong 及其余消息忽略
                    Some(Err(_)) => return true,
                }
            }
        };
        tokio::time::timeout(budget, fut).await.unwrap_or(false)
    }

    /// 发送一条 binary（不 panic：连接已被对端关闭等失败返回 false）。
    pub async fn try_send_binary(&mut self, data: &[u8]) -> bool {
        self.ws
            .send(WsMessage::Binary(data.to_vec().into()))
            .await
            .is_ok()
    }

    async fn recv_with_timeout(&mut self) -> Recv {
        let fut = async {
            loop {
                match self.ws.next().await {
                    None => return Recv::Close,
                    Some(Ok(WsMessage::Binary(b))) => return Recv::Binary(b.to_vec()),
                    Some(Ok(WsMessage::Close(_))) => return Recv::Close,
                    Some(Ok(_)) => continue, // Ping/Pong 等忽略
                    Some(Err(_)) => return Recv::Close,
                }
            }
        };
        // 超时即 panic：连接既未收到数据也未关闭本身就是待暴露的缺陷，
        // 伪装成 Close 会掩盖真实问题。
        tokio::time::timeout(Duration::from_secs(5), fut)
            .await
            .expect("recv 超时（5s）：连接既无数据也无关闭")
    }
}

/// 原始 HTTP GET（async 版，进程内 server 用）。
pub async fn raw_http_get(addr: SocketAddr, path: &str) -> RawHttpResponse {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.expect("TCP 连接失败");
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("发送 HTTP 请求失败");
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .expect("读取 HTTP 响应失败");
    parse_status_line(&buf)
}

/// 原始 HTTP GET（阻塞版，子进程测试用；连接失败返回 status 0 = 未就绪）。
pub fn blocking_raw_http_get(addr: SocketAddr, path: &str) -> RawHttpResponse {
    use std::io::{Read, Write};
    let Ok(mut stream) = std::net::TcpStream::connect_timeout(
        &addr,
        std::time::Duration::from_millis(500),
    ) else {
        return RawHttpResponse { status: 0 };
    };
    if stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .is_err()
    {
        return RawHttpResponse { status: 0 };
    }
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).is_err() {
        return RawHttpResponse { status: 0 };
    }
    parse_status_line(&buf)
}

fn parse_status_line(buf: &[u8]) -> RawHttpResponse {
    let head = String::from_utf8_lossy(buf);
    let first_line = head.lines().next().unwrap_or_default();
    let status = first_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    RawHttpResponse { status }
}
