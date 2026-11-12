//! 子进程管理: 程序退出、超时、请求取消时结束 yt-dlp 以及它启动的 ffmpeg,不留下孤儿进程
//!
//! Unix 下每个 yt-dlp 放在单独的进程组里(进程组ID = yt-dlp 的PID),结束时向整个进程组发信号,
//! 这样 yt-dlp 启动的 ffmpeg 也会一起结束(tokio 的 kill_on_drop 只能结束 yt-dlp 本身)。
//! 单独的进程组也让终端里的 Ctrl+C 不会直接发给子进程,统一由本程序处理。

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use tokio::process::{Child, Command};

/// 正在运行的子进程(进程组ID)
static RUNNING: LazyLock<Mutex<HashSet<u32>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// 启动前设置: 放到单独的进程组
pub fn prepare(cmd: &mut Command) {
    #[cfg(unix)]
    cmd.process_group(0);
    cmd.kill_on_drop(true);
}

/// 子进程守卫: 没有正常结束就被丢弃时(超时、请求取消、程序退出),结束整个进程组
pub struct ChildGuard {
    pid: Option<u32>,
    finished: bool,
}

impl ChildGuard {
    pub fn new(child: &Child) -> Self {
        let pid = child.id();
        if let Some(pid) = pid {
            RUNNING.lock().unwrap().insert(pid);
        }
        Self { pid, finished: false }
    }

    /// 子进程已经结束(已经回收,进程组ID可能被重新使用,不能再发信号)
    pub fn finish(mut self) {
        self.finished = true;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(pid) = self.pid else {
            return;
        };
        RUNNING.lock().unwrap().remove(&pid);
        if !self.finished {
            kill_group(pid, Signal::Kill);
        }
    }
}

#[derive(Clone, Copy)]
pub enum Signal {
    /// 通知结束
    Term,

    /// 强制结束
    Kill,
}

/// 结束所有正在运行的子进程(程序退出时调用)
/// 返回结束的数量
pub fn kill_all(signal: Signal) -> usize {
    let pids: Vec<u32> = RUNNING.lock().unwrap().iter().copied().collect();
    for pid in &pids {
        kill_group(*pid, signal);
    }
    pids.len()
}

#[cfg(unix)]
fn kill_group(pid: u32, signal: Signal) {
    let sig = match signal {
        Signal::Term => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
    };

    // SAFETY: killpg 只是发送信号,参数都是普通整数
    unsafe {
        libc::killpg(pid as libc::pid_t, sig);
    }
}

/// Windows 没有进程组,yt-dlp 由 kill_on_drop 结束
#[cfg(not(unix))]
fn kill_group(_pid: u32, _signal: Signal) {}
