//! Windows Job Object 封装。仅在 `target_os = "windows"` 下编译。
//!
//! 直接使用 `windows-sys` 0.52 FFI 调用：
//! - `CreateJobObjectW` —— 匿名 Job
//! - `SetInformationJobObject` (JobObjectExtendedLimitInformation) —— 设置 limit
//! - `AssignProcessToJobObject` —— 绑定进程
//! - `OpenProcess` + `CloseHandle` —— 拿到进程 handle
//!
//! 强制设置的不变量：
//! - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` = **on**
//! - `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` = **off**（不在 LimitFlags bitmask 中）
//! - `JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION` = on
//! - `JOB_OBJECT_LIMIT_PROCESS_MEMORY` / `JOB_OBJECT_LIMIT_JOB_MEMORY`
//! - `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`
//!
//! `JobHandle` 析构会 `CloseHandle`，触发 KILL_ON_JOB_CLOSE。

#![cfg(target_os = "windows")]

use crate::sandbox::SandboxError;

use std::mem;
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION,
    JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// 默认 Job Object 限制。
#[derive(Debug, Clone, Copy)]
pub struct JobLimits {
    pub process_memory_bytes: usize,
    pub job_memory_bytes: usize,
    pub active_process_limit: u32,
}

impl Default for JobLimits {
    fn default() -> Self {
        Self {
            process_memory_bytes: 2 * 1024 * 1024 * 1024,
            job_memory_bytes: 4 * 1024 * 1024 * 1024,
            active_process_limit: 32,
        }
    }
}

/// 持有 Job Object 资源；drop 时 close handle → 子进程被 KILL_ON_JOB_CLOSE 带走。
pub struct JobHandle {
    handle: HANDLE,
}

// Job HANDLE 本质上是 isize 数值，跨线程传递无内部共享状态。
unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if self.handle != 0 && self.handle != INVALID_HANDLE_VALUE {
            // SAFETY: handle 在构造时为 CreateJobObjectW 返回的有效 HANDLE；
            // Drop 中唯一持有，关闭一次安全。
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

impl std::fmt::Debug for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobHandle").finish()
    }
}

impl JobHandle {
    /// 原始 handle —— 仅供 `CreateProcessAsUserW + STARTUPINFOEXW` 路径使用。
    pub fn raw(&self) -> HANDLE {
        self.handle
    }

    /// 把已存在的进程加入本 Job。
    pub fn assign_process(&self, pid: u32) -> Result<(), SandboxError> {
        // SAFETY: FFI 调用 OpenProcess，失败返回 0；成功返回有效 HANDLE，
        // 必须在使用后 CloseHandle。
        let proc_handle = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, FALSE, pid) };
        if proc_handle == 0 {
            return Err(SandboxError::WindowsApi(format!(
                "OpenProcess(pid={pid}) failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // SAFETY: 两个 handle 都是有效的内核对象引用。AssignProcessToJobObject
        // 返回 BOOL：非零成功。
        let ok = unsafe { AssignProcessToJobObject(self.handle, proc_handle) };

        // SAFETY: proc_handle 是上面 OpenProcess 返回的有效 HANDLE。
        // AssignProcessToJobObject 在内核中已经复制了关联，我们可以安全释放外部句柄。
        unsafe {
            CloseHandle(proc_handle);
        }

        if ok == 0 {
            return Err(SandboxError::WindowsApi(format!(
                "AssignProcessToJobObject(pid={pid}) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    /// 查询当前 limit info —— 用于测试 assertion。
    pub fn query_limit_info(&self) -> Result<JOBOBJECT_EXTENDED_LIMIT_INFORMATION, SandboxError> {
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
        let mut returned: u32 = 0;
        // SAFETY: handle 有效；info 是栈上有效内存；returned 是有效指针。
        let ok = unsafe {
            QueryInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                &mut returned,
            )
        };
        if ok == 0 {
            return Err(SandboxError::WindowsApi(format!(
                "QueryInformationJobObject failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(info)
    }

    /// 是否仍是有效 Job handle（通过一次 query 间接验证）。
    pub fn is_valid(&self) -> bool {
        self.query_limit_info().is_ok()
    }
}

/// 创建一个新的 Job Object，并把所有强约束 flag 一次性设到位。
pub fn create_sandbox_job() -> Result<JobHandle, SandboxError> {
    create_sandbox_job_with_limits(JobLimits::default())
}

pub fn create_sandbox_job_with_limits(limits: JobLimits) -> Result<JobHandle, SandboxError> {
    // SAFETY: 传 null 名 (匿名 Job) + null SECURITY_ATTRIBUTES → 安全。
    let job_handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if job_handle == 0 {
        return Err(SandboxError::WindowsApi(format!(
            "CreateJobObjectW failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let handle = JobHandle { handle: job_handle };

    // 构造 JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
    let flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_PROCESS_MEMORY
        | JOB_OBJECT_LIMIT_JOB_MEMORY;
    info.BasicLimitInformation.LimitFlags = flags;
    info.BasicLimitInformation.ActiveProcessLimit = limits.active_process_limit;
    info.ProcessMemoryLimit = limits.process_memory_bytes;
    info.JobMemoryLimit = limits.job_memory_bytes;

    // SAFETY: handle 有效；info 指向有效栈内存；size 匹配。
    let ok = unsafe {
        SetInformationJobObject(
            handle.handle,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        return Err(SandboxError::WindowsApi(format!(
            "SetInformationJobObject failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // 防御性 sanity check：SILENT_BREAKAWAY_OK 不能被设置
    debug_assert_eq!(
        info.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
        0,
        "JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK must NOT be set"
    );

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sandbox_job_succeeds() {
        let job = create_sandbox_job().expect("create job");
        assert!(job.is_valid());
    }

    #[test]
    fn limit_flags_set_kill_on_close_and_no_silent_breakaway() {
        let job = create_sandbox_job().expect("create");
        let info = job.query_limit_info().expect("query");
        let flags = info.BasicLimitInformation.LimitFlags;
        assert_ne!(
            flags & JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            0,
            "KILL_ON_JOB_CLOSE must be on"
        );
        assert_eq!(
            flags & JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
            0,
            "SILENT_BREAKAWAY_OK must be off"
        );
    }
}
