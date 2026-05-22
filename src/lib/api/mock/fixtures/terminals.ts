import type { TerminalCandidate } from "../../contracts";

export function windowsTerminals(): TerminalCandidate[] {
  return [
    {
      id: "wt",
      kind: "wt",
      displayName: "Windows Terminal",
      path: "C:\\Users\\you\\AppData\\Local\\Microsoft\\WindowsApps\\wt.exe",
      installed: true,
      isDefault: true,
    },
    {
      id: "cmd",
      kind: "cmd",
      displayName: "命令提示符 (cmd)",
      path: "C:\\Windows\\System32\\cmd.exe",
      installed: true,
      isDefault: false,
    },
    {
      id: "powershell",
      kind: "powershell",
      displayName: "PowerShell",
      path: "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
      installed: true,
      isDefault: false,
    },
  ];
}

export function macTerminals(): TerminalCandidate[] {
  return [
    {
      id: "terminal-app",
      kind: "terminal-app",
      displayName: "Terminal.app",
      path: "/System/Applications/Utilities/Terminal.app",
      installed: true,
      isDefault: true,
    },
    {
      id: "iterm2",
      kind: "iterm2",
      displayName: "iTerm2",
      path: "/Applications/iTerm.app",
      installed: false,
      isDefault: false,
    },
  ];
}

export function noTerminals(): TerminalCandidate[] {
  return [];
}
