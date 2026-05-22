//! Windows Restricted Token：从当前进程 token 派生一个受限 token，去掉敏感权限并降
//! 完整性级别。子进程用该 token 启动 (`CreateProcessAsUserW`) 即可被强制限制。
//!
//! 仅在 `target_os = "windows"` 下编译。
//!
//! ## 三个核心 flag（CreateRestrictedToken）
//! - `DISABLE_MAX_PRIVILEGE` (0x01) — 删除除 SeChangeNotifyPrivilege 外所有权限。
//! - `LUA_TOKEN` (0x04) — 强制 Medium IL；子进程无法 UAC 提权。
//! - `WRITE_RESTRICTED` (0x08) — 只能写显式允许的 restricted-SID 名下的对象。
//!
//! MVP 使用前两个；`WRITE_RESTRICTED` 需要配合 DACL 设置，留到 v2。

#![cfg(target_os = "windows")]

use crate::sandbox::SandboxError;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::{
    CreateRestrictedToken, DISABLE_MAX_PRIVILEGE, LUA_TOKEN, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
    TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// 包装一个受限 token handle；drop 时自动关闭。
pub struct TokenHandle {
    handle: HANDLE,
}

impl TokenHandle {
    /// 拿到原始 HANDLE 值（用于后续 CreateProcessAsUserW）。
    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for TokenHandle {
    fn drop(&mut self) {
        if self.handle != 0 && self.handle != INVALID_HANDLE_VALUE {
            // SAFETY: handle 在 new 时是 CreateRestrictedToken 返回的有效 token；
            // 此处仅在 Drop 中且唯一持有，关闭一次是安全的。
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

/// 创建受限 token：基于当前进程 token，删除最高权限 + 降到 Medium IL。
///
/// 返回的 token 可以传给 `CreateProcessAsUserW` 来启动受限子进程。
pub fn create_restricted_token() -> Result<TokenHandle, SandboxError> {
    // 1. 取当前进程 token（只读 + 复制 + 分配为主 token 权限）
    let mut current_token: HANDLE = 0;
    // SAFETY:
    // - GetCurrentProcess 返回伪句柄（-1），无须释放，安全。
    // - OpenProcessToken 接受 process handle / access mask / out-handle 指针。
    let ok = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY,
            &mut current_token,
        )
    };
    if ok == 0 {
        return Err(SandboxError::WindowsApi(format!(
            "OpenProcessToken failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // RAII 包装，保证早期返回时仍能 close
    let _current_token_guard = TokenHandle {
        handle: current_token,
    };

    // 2. CreateRestrictedToken 派生受限 token
    let mut restricted_token: HANDLE = 0;
    let flags = DISABLE_MAX_PRIVILEGE | LUA_TOKEN;

    // SAFETY:
    // - 我们已经持有有效的 current_token。
    // - 所有 SID/Privilege 数组传 null + 长度 0（表示不额外限制）。
    // - out 指针 restricted_token 指向我们栈上的有效内存。
    let ok = unsafe {
        CreateRestrictedToken(
            current_token,
            flags,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            &mut restricted_token,
        )
    };
    if ok == 0 {
        return Err(SandboxError::WindowsApi(format!(
            "CreateRestrictedToken failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    // drop(_current_token_guard) 在函数结束时自动 close

    Ok(TokenHandle {
        handle: restricted_token,
    })
}

/// 占位：DACL 文件夹白名单。目前仅记录意图；v2 实装。
///
/// 入参 `_cwd`：仅允许该目录子树可写。
pub fn apply_workdir_dacl(_cwd: &std::path::Path) -> Result<(), SandboxError> {
    // MVP: 不强制 DACL；由 L1/L2 命令字符串拦截兜底。
    Ok(())
}

// 让 TokenHandle 在跨线程传递时安全（仅承载 HANDLE 数值，无内部共享状态）。
unsafe impl Send for TokenHandle {}
// 不实现 Sync —— HANDLE 应被独占持有。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_restricted_token_succeeds() {
        // 注：本测试需要标准用户权限；在 SYSTEM 上下文可能失败。CI 一般跑得通。
        let token = create_restricted_token().expect("create restricted token");
        assert_ne!(token.raw(), 0);
        assert_ne!(token.raw(), INVALID_HANDLE_VALUE);
    }

    #[test]
    fn token_drops_without_panic() {
        let token = create_restricted_token().expect("create");
        drop(token);
    }
}
