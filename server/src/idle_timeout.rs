//! 连接级双向空闲超时（Story 15.4 二轮评审修复 #2，F11「保留 idle 超时」）。
//!
//! 语义：**同一连接上读且写均静默超过 [`IDLE_TIMEOUT_SECS`] 秒才断开**。
//!
//! - 读活动（`poll_read` 返回 Ready）或写活动（`poll_write` 返回 Ready）
//!   任一发生即重置 deadline——SSE 的 30s KeepAlive 心跳是写活动，
//!   **健康 SSE 连接永不触发空闲断开**；
//! - **响应超时保持禁用**（chat 分钟级挂起是正常态）——超时只针对
//!   连接静默，不针对单请求在途时长；
//! - Pending 轮询期间 Sleep 的 waker 同步注册（连接完全静默、hyper
//!   不再回调时，Sleep 到期仍能唤醒任务返回 TimedOut）。
//!
//! 实现：自定义 [`Listener`] 包装（axum::serve 兼容——`tap_io` 官方
//! 先例同款形态），每条被接受连接的 IO 经 [`IdleTimeoutStream`] 包装；
//! 优雅停机与 8s 兜底语义不受影响（`poll_shutdown` 直接透传，不走
//! 超时判定）。

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::serve::Listener;
use tokio::io::ReadBuf;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Sleep;

/// 空闲断开阈值（秒）：读且写均静默超过此时长才断开。
///
/// 值选取：远大于 SSE KeepAlive 心跳间隔（30s×4 余量——偶发丢一两拍
/// 心跳不误杀），又足以回收真正死滞的连接。
pub const IDLE_TIMEOUT_SECS: u64 = 120;

/// TCP 连接空闲超时包装：读/写任一活动重置计时器，双静默到期断开。
pub struct IdleTimeoutStream {
    inner: TcpStream,
    idle: Duration,
    /// 当前 deadline 的计时器（Ready 活动时 reset）。
    timeout: Pin<Box<Sleep>>,
}

impl IdleTimeoutStream {
    fn new(inner: TcpStream, idle: Duration) -> Self {
        Self {
            inner,
            idle,
            timeout: Box::pin(tokio::time::sleep(idle)),
        }
    }

    /// 判定是否已空闲到期：到期 ⇒ `Err(TimedOut)`（连接层关闭信号）。
    fn check_deadline(&mut self, cx: &mut Context<'_>) -> io::Result<()> {
        if self.timeout.as_mut().poll(cx).is_ready() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("连接空闲超过 {}s（读写双静默）", self.idle.as_secs()),
            ));
        }
        Ok(())
    }

    /// 活动发生：重置 deadline。
    fn reset_deadline(&mut self) {
        self.timeout
            .as_mut()
            .reset(tokio::time::Instant::now() + self.idle);
    }
}

impl tokio::io::AsyncRead for IdleTimeoutStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // 先判到期（连接静默期 hyper 停止轮询时，Sleep 的 waker 仍会
        // 唤醒本任务走到这里）
        if let Err(e) = self.check_deadline(cx) {
            return Poll::Ready(Err(e));
        }
        match Pin::new(&mut self.inner).poll_read(cx, buf) {
            // 读到数据：活动，重置
            Poll::Ready(r) => {
                self.reset_deadline();
                Poll::Ready(r)
            }
            // 无数据：Pending（Sleep waker 已在 check_deadline 注册，
            // 到期即唤醒）
            Poll::Pending => Poll::Pending,
        }
    }
}

impl tokio::io::AsyncWrite for IdleTimeoutStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if let Err(e) = self.check_deadline(cx) {
            return Poll::Ready(Err(e));
        }
        match Pin::new(&mut self.inner).poll_write(cx, buf) {
            // 写出字节（含 KeepAlive 心跳）：活动，重置
            Poll::Ready(r) => {
                self.reset_deadline();
                Poll::Ready(r)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// 优雅停机路径直接透传（不走超时判定——关闭是显式意图）。
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}

/// 空闲超时监听器：包装 [`TcpListener`]，接受的每条连接套上
/// [`IdleTimeoutStream`]。
pub struct IdleTimeoutListener {
    inner: TcpListener,
    idle: Duration,
}

impl IdleTimeoutListener {
    pub fn new(inner: TcpListener, idle: Duration) -> Self {
        Self { inner, idle }
    }
}

impl Listener for IdleTimeoutListener {
    type Io = IdleTimeoutStream;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        // 错误处理对齐 TcpListener 的 Listener 实现（日志 + 重试）
        loop {
            match self.inner.accept().await {
                Ok((stream, addr)) => {
                    return (IdleTimeoutStream::new(stream, self.idle), addr)
                }
                Err(e) => {
                    handle_accept_error(e).await;
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

/// accept 错误处理：连接数耗尽等瞬态错误退避重试（axum 同款语义）。
async fn handle_accept_error(e: io::Error) {
    match e.kind() {
        io::ErrorKind::ConnectionAborted
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::Interrupted => {}
        _ => {
            tracing::error!("接受连接失败: {}", e);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// 连接信息载体：`into_make_service_with_connect_info::<RemoteAddr>()`
/// 的连接信息源。
///
/// 为什么不是直接 `SocketAddr`：`Connected` 是 axum 的 trait、
/// `SocketAddr` 与 `IncomingStream` 均为外部类型——孤儿规则禁止
/// `impl Connected<IncomingStream<MyListener>> for SocketAddr`；本地
/// newtype 即合法（axum 为 TcpListener 提供同款实现的 crate 内镜像）。
#[derive(Clone, Copy, Debug)]
pub struct RemoteAddr(pub std::net::SocketAddr);

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, IdleTimeoutListener>>
    for RemoteAddr
{
    fn connect_info(
        stream: axum::serve::IncomingStream<'_, IdleTimeoutListener>,
    ) -> Self {
        RemoteAddr(*stream.remote_addr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// 真实 TCP 对（loopback）返回包装后的服务端流 + 客户端流。
    async fn tcp_pair(idle: Duration) -> (IdleTimeoutStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        (IdleTimeoutStream::new(server, idle), client)
    }

    /// 双向静默 ⇒ 到期断开（TimedOut）：Pending 轮询期间 Sleep waker
    /// 已注册，连接完全静默时超时仍能触发。
    #[tokio::test]
    async fn silence_exceeding_idle_is_disconnected() {
        let (mut wrapped, _client) = tcp_pair(Duration::from_millis(200)).await;
        let mut buf = [0u8; 8];
        let start = std::time::Instant::now();
        let result = wrapped.read(&mut buf).await;
        let elapsed = start.elapsed();
        let err = result.expect_err("静默超时必须 Err");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "错误类型: {}", err);
        // 到期时机：阈值附近（含调度余量），不得立即、不得远超
        assert!(
            elapsed >= Duration::from_millis(200),
            "不得早于阈值断开（实得 {:?}）",
            elapsed
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "断开不得拖沓（实得 {:?}）",
            elapsed
        );
    }

    /// 读活动重置计时器：持续读（< 阈值间隔）超过总阈值不断开。
    #[tokio::test]
    async fn read_activity_resets_deadline() {
        // 阈值 300ms；客户端每 100ms 写一字节 ⇒ 服务端读侧持续活动
        let (mut wrapped, mut client) = tcp_pair(Duration::from_millis(300)).await;
        let writer = tokio::spawn(async move {
            for _ in 0..6 {
                client.write_all(b"x").await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        let mut buf = [0u8; 8];
        // 连续读 6 次：总时长 ~600ms > 300ms 阈值——每次读活动重置
        for _ in 0..6 {
            let n = wrapped
                .read(&mut buf)
                .await
                .expect("读活动期间不得空闲断开");
            assert_eq!(n, 1);
        }
        writer.await.unwrap();
    }

    /// 写活动重置计时器（SSE KeepAlive 心跳语义）：持续写（< 阈值间隔）
    /// 超过总阈值不断开。
    #[tokio::test]
    async fn write_activity_resets_deadline() {
        // 阈值 300ms；每 100ms 写一字节 × 6（总 600ms > 阈值）——若无
        // 重置，第 4 次写会撞初始 deadline
        let (mut wrapped, _client) = tcp_pair(Duration::from_millis(300)).await;
        for _ in 0..6 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            wrapped
                .write_all(b"x")
                .await
                .expect("持续写活动（心跳语义）不得空闲断开");
        }
    }
}
