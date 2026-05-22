import type { LocalizedString, TypedError } from "../../contracts";

export const messages = {
  installerProbingRegistry: {
    zh: "正在选择镜像源...",
    en: "Probing npm registries...",
    ja: "npm レジストリを探索中...",
  },
  installerInstallingNode: {
    zh: "正在安装 Node.js 20 LTS...",
    en: "Installing Node.js 20 LTS...",
    ja: "Node.js 20 LTS をインストール中...",
  },
  installerInstallingCli: {
    zh: "正在安装 CLI...",
    en: "Installing CLI...",
    ja: "CLI をインストール中...",
  },
  installerValidating: {
    zh: "正在校验安装结果...",
    en: "Validating installation...",
    ja: "インストール結果を検証中...",
  },
  installerCompleted: {
    zh: "安装完成",
    en: "Installation completed",
    ja: "インストール完了",
  },
  installerFailedNetwork: {
    zh: "无法连接到任何 npm 镜像源",
    en: "Unable to reach any npm registry",
    ja: "npm レジストリへの接続に失敗しました",
  },
  installerFailedNodeMissing: {
    zh: "未检测到 Node.js，请先安装 Node.js",
    en: "Node.js not found. Please install Node.js first.",
    ja: "Node.js が見つかりません。先に Node.js をインストールしてください。",
  },
  fixStarting: {
    zh: "准备中...",
    en: "Preparing...",
    ja: "準備中...",
  },
  fixRunning: {
    zh: "正在修复...",
    en: "Applying fix...",
    ja: "修復中...",
  },
  fixValidating: {
    zh: "正在校验...",
    en: "Validating...",
    ja: "検証中...",
  },
  fixCompleted: {
    zh: "修复完成",
    en: "Fix completed",
    ja: "修復完了",
  },
  uninstallSuccess: {
    zh: "已卸载",
    en: "Uninstalled",
    ja: "アンインストールしました",
  },
  switchSuccess: {
    zh: "Profile 已切换，下次启动 CLI 时生效",
    en: "Profile switched. Takes effect on next CLI launch.",
    ja: "プロファイルを切り替えました。次回の CLI 起動時に有効になります。",
  },
  launchSuccess: {
    zh: "CLI 已在系统终端启动",
    en: "CLI launched in system terminal",
    ja: "システムターミナルで CLI を起動しました",
  },
  launchFailedNoTerminal: {
    zh: "未检测到可用的系统终端",
    en: "No system terminal available",
    ja: "利用可能なシステムターミナルが見つかりません",
  },
  launchFailedProfileMissing: {
    zh: "目标 Profile 不存在",
    en: "Target profile not found",
    ja: "対象のプロファイルが見つかりません",
  },
  sandboxRedlineBlocked: {
    zh: "命令命中 L2 硬红线，永不可执行",
    en: "Command blocked by L2 hard redline (permanently disallowed)",
    ja: "L2 ハードレッドライン違反のためコマンドはブロックされました",
  },
  notFound: {
    zh: "未找到",
    en: "Not found",
    ja: "見つかりません",
  },
} satisfies Record<string, LocalizedString>;

export const errors = {
  networkUnreachable: {
    code: "NETWORK_UNREACHABLE",
    message: messages.installerFailedNetwork,
    retryable: true,
  },
  nodeMissing: {
    code: "NODE_MISSING",
    message: messages.installerFailedNodeMissing,
    retryable: false,
  },
  profileNotFound: {
    code: "PROFILE_NOT_FOUND",
    message: messages.launchFailedProfileMissing,
    retryable: false,
  },
  noTerminal: {
    code: "NO_TERMINAL_AVAILABLE",
    message: messages.launchFailedNoTerminal,
    retryable: false,
  },
  l2RedlineBlocked: {
    code: "L2_REDLINE_BLOCKED",
    message: messages.sandboxRedlineBlocked,
    retryable: false,
  },
} satisfies Record<string, TypedError>;
