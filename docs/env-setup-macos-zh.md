# CC Launcher 环境配置完整路线（macOS）

> 这是一份"完全不懂电脑也能照做"的 macOS 路线书。
> Windows 用户请看：[env-setup-windows-zh.md](./env-setup-windows-zh.md)

---

## 0. 目标受众

这份文档写给：

- 完全小白：**不懂 AI / 不懂编程 / 不懂"什么叫 Terminal.app"**
- 只想点一个图标就能用上 Claude Code（Anthropic 出的 AI 编程助手）和 Codex CLI（OpenAI 出的 AI 编程助手）
- 担心装完一堆软件会把电脑搞坏 / 怕被勒索病毒清盘

阅读本文不需要你提前知道任何技术名词。所有专业词都会在出现的时候用大白话括号解释。

---

## 1. 准备阶段（30 秒检查）

点击屏幕左上角苹果 logo → "关于本机"，对照下表看看你的 Mac 达不达标。

| 检查项 | 绿色（够用） | 黄色（能装但慢） | 红色（装不了） |
|---|---|---|---|
| **macOS 版本** | macOS 12 (Monterey) 或更高（Ventura / Sonoma / Sequoia 等） | macOS 11 (Big Sur) | macOS 10.15 (Catalina) 及以下 |
| **CPU 架构** | Apple Silicon（M1 / M2 / M3 / M4）或 Intel x64 | 都行 | 老款 PowerPC（早就停产，几乎不存在） |
| **CPU 核心数** | 4 核及以上（任何 M 系列芯片都满足） | 2-3 核（早期 Intel 双核机） | 1 核（基本买不到了） |
| **内存（RAM）** | 8 GB 及以上 | 4-8 GB | 不到 4 GB |
| **磁盘剩余空间** | 5 GB 以上 | 2-5 GB | 不到 2 GB |
| **网络** | 能打开 [百度](https://www.baidu.com) 和 [GitHub](https://github.com) 任一个 | 只能打开百度（国内镜像够用） | 完全无网 |

> 📸 截图：苹果菜单 → 关于本机 → 概览页面，红框框出"芯片" / "内存" / "macOS 版本"三行

**怎么看磁盘剩余空间**：苹果菜单 → 关于本机 → "更多信息" → "储存空间"，看可用空间多少 GB。

**怎么知道是 Apple Silicon 还是 Intel**：苹果菜单 → 关于本机 → 看"芯片"行：

- 写 "Apple M1" / "Apple M2" / "Apple M3" / "Apple M4" → 你是 **Apple Silicon (arm64)**
- 写 "Intel Core i5/i7/i9..." → 你是 **Intel (x64)**

这个区别**很重要**，后面装 Node 会自动按你的架构挑对应的包。

如果你哪一项是红色，先解决它再继续。比如磁盘不够就清理"下载"、"废纸篓"、旧的 Xcode 缓存。

---

## 2. CC Launcher 安装

### 2.1 下载

打开 CC Launcher 的 Releases 页面（项目发布页面），找到最新版本，下载文件名形如：

```
cc-launcher_<版本号>_aarch64_zh-CN.dmg    （Apple Silicon M1/M2/M3/M4 用）
cc-launcher_<版本号>_x64_zh-CN.dmg        （Intel Mac 用）
```

> Releases 页面会随项目发布更新，请到项目首页查看最新地址。

`.dmg` 是 macOS 的标准磁盘镜像安装包，双击会挂载成一个虚拟磁盘，里面有 `.app` 文件。

> 📸 截图：浏览器下载完成后，访达里 .dmg 文件的样子

### 2.2 安装步骤

1. **双击下载好的 .dmg 文件** — 会出现一个磁盘镜像窗口，里面有 `CC Launcher.app` 和 "Applications" 文件夹的快捷方式
2. **把 `CC Launcher.app` 拖到 "Applications" 文件夹**（这是 macOS 标准安装动作）
3. **首次启动会被 Gatekeeper（macOS 的应用验证机制）拦截**，弹出 "无法验证开发者" 警告
   - 这不代表病毒，是因为 macOS 默认只允许从 App Store 装应用
   - 解决方法：到"系统设置 → 隐私与安全性"，滚动到最下面会看到"已阻止 CC Launcher.app 的使用"，点旁边的**"仍要打开"**按钮
   - 之后会再弹一个确认对话框，再点"打开"即可
4. **不需要输入管理员密码**（CC Launcher 完全装在 `/Applications/` 用户级目录）
5. **以后再启动就不会再弹拦截**

> 📸 截图：Gatekeeper 警告对话框 + 系统设置里"仍要打开"按钮位置

### 2.3 首次启动 Onboarding（引导）

第一次双击 CC Launcher 启动后，会出现：

1. **欢迎页** — 一段中文说明"我们是谁、我们能做什么"
2. **环境自检页** — 后台自动跑探测（详见第 3 节），不需要你做任何事
3. **安全提示页** — 告诉你"默认沙盒严格，不会清盘"
4. **首个 Profile 创建引导** — 走到第 6 节再细看

### 2.4 安全提示

CC Launcher 默认开启**双层沙盒**（详见第 7 节）：

- **不会清盘** — 即使 AI 自作主张敲 `rm -rf /` / `sudo rm -rf ~` 这种危险命令，硬红线层会直接拒绝
- **不会改你的系统文件** — `/etc/hosts` / `/Library/LaunchDaemons/` / `/System/` 都被锁死
- **不会偷偷修改其他位置** — CLI 只能在 `~/cc-launcher-projects/` 内动手

唯一会"动你电脑"的操作是：

- 在 `~/Library/Application Support/cc-switch/` 下安装一份 Node.js 私有运行时
- 在 `~/.cc-switch/` 下存放配置和日志

这两个目录**全部位于你的用户主目录下**，不需要 sudo，也不会动 `/usr/local`、Homebrew、系统 Python 等其他位置。

---

## 3. 自动环境探测（CC Launcher 自动跑）

CC Launcher 启动后会用 1-3 秒跑完 **17 项探测**。每一项都用绿 / 黄 / 红三色标记。下面按重要性分组解释。

### 3.1 必需项（红色会阻塞启动）

| 探测项 | 探测什么 | 绿色含义 | 黄色含义 | 红色会怎样 |
|---|---|---|---|---|
| **Node.js 可执行**（node 命令） | 系统里有没有 Node（npm 的运行环境，类似 Python 之于 pip） | 系统已装 Node 20 或更高 | 装了 Node 18-19（能用，但 CC Launcher 会用私有 20） | 没装 → **CC Launcher 自动给你装一份私有的，不动系统**（详见第 4 节） |
| **npm 可执行**（包管理工具） | npm 命令在不在 | npm 10 及以上 | npm 9 | 没装 → 跟着 Node 一起装 |
| **Git 可执行**（代码版本管理工具） | Git 在不在 | 任何现代版本 | — | 没装 → **CC Launcher 弹"一键装 Git"按钮**，会触发 `xcode-select --install` 系统对话框（详见 3.3 节） |
| **磁盘剩余空间** | 用户主目录所在卷还剩多少 | 大于 5 GB | 2-5 GB | 不到 2 GB → 不允许装东西，弹"打开访达清理"按钮 |
| **网络可达性** | 能不能连上 npm 软件源（4 个候选服务器，详见第 5 节） | 至少 1 个延迟低于 1 秒 | 全都通但都大于 1 秒 | 4 个全连不上 → "网络不通"提示，所有联网按钮变灰 |
| **环境变量冲突** | 你之前装过其他 AI 工具留下的 `ANTHROPIC_*` / `OPENAI_*` / `GEMINI_*` 变量（包括 `~/.zshrc` / `~/.bash_profile` 里的 export） | 没有冲突 | — | 有冲突 → 弹"一键清理 + 自动备份"按钮 |
| **工作目录可写性** | `~/cc-launcher-projects/` 能不能创建/写入 | 可写 | — | 不可写 → 自动尝试创建，失败则提示权限问题 |

### 3.2 建议项（黄色提醒，不阻塞）

| 探测项 | 探测什么 | 绿色 | 黄色 | 红色 |
|---|---|---|---|---|
| **OS 版本** | macOS 是哪个大版本 | macOS 12+ | macOS 11 | macOS 10.x |
| **CPU 核心数** | 物理核数 | 大于等于 4 | 2-3 | 1 |
| **总内存** | 装机内存 | 大于等于 8 GB | 4-8 GB | 不到 4 GB |
| **可用内存** | 当前剩余 RAM | 大于 2 GB | 1-2 GB | 不到 1 GB（提示关掉其他程序） |
| **PATH 完整性** | 系统 PATH 里有没有 CC Launcher 需要的关键目录 | 都在 | 部分缺失 | 关键缺失 → Launcher 自动在子进程内注入，不动你的 shell 配置 |

### 3.3 信息项（macOS 特有）

下面这些是 macOS 系统级权限相关因素。MVP 阶段**只提示不自动改**。

| 探测项 | 探测什么 | 出现警告时 CC Launcher 会怎样 |
|---|---|---|
| **管理员状态** | 你是不是用 `sudo` 启动了 Launcher（`geteuid() == 0`） | 弹"无需以 root 运行"提示，建议你以普通账户启动 |
| **Rosetta 已装**（仅 Apple Silicon） | `/usr/bin/pgrep -q oahd`（oahd 进程存在 = Rosetta 2 已装） | 如果你是 Apple Silicon 且 CC Launcher 需要装 x64 Node（极少情况）会弹"运行 `softwareupdate --install-rosetta --agree-to-license` 安装 Rosetta"提示。MVP 默认使用 arm64 Node，**通常不需要 Rosetta** |
| **xcode-select / Command Line Tools** | Git 来自 Xcode Command Line Tools。该工具集装了没 | 没装 → 触发 `xcode-select --install`，弹苹果系统对话框，点"安装"等几分钟即可。Git / clang / make 等基础工具一起装 |
| **Gatekeeper 隔离属性** | 你手动下载的二进制有没有被 `com.apple.quarantine` 隔离属性标记 | 信息性提示。CC Launcher 自身已经走苹果开发者签名 + 公证（notarize），不会被 Gatekeeper 拦 |
| **系统代理** | `HTTP_PROXY` / `HTTPS_PROXY` 环境变量 + `scutil --proxy` 读 macOS 系统代理设置 | 如果设了，自动透传给 npm。如果代理不可达，提示你检查 |

> 📸 截图：CC Launcher 主界面"系统自检"卡片，绿/黄/红分组样式

---

## 4. Node.js 私有 runtime 安装（Launcher 自动）

### 4.1 为什么用私有 runtime

如果你的 Mac 已经装了 Node.js（比如以前学过编程，或者用 Homebrew 装过），CC Launcher 仍然**优先用自己的私有版本**。原因：

1. **不污染系统**：CC Launcher 卸载时一个 `rm -rf` 就能干净清空，不留尾巴
2. **不动 Homebrew / nvm / volta**：你的其他项目继续用你原来的 Node
3. **不需要 sudo**：装到用户目录，不弹密码框
4. **统一基线**：所有 CC Launcher 用户跑的都是同一份 Node 20 LTS，少踩兼容性的坑
5. **不改 shell rc**：不会偷偷往 `~/.zshrc` / `~/.bash_profile` 里塞 export 语句

### 4.2 落点路径（固定）

```
~/Library/Application Support/cc-switch/runtime/node/
```

里面会有：

```
bin/
  node
  npm
  npx
lib/
include/
share/
... 等等
```

### 4.3 Node 20 LTS 安装包来源

CC Launcher 会从以下 4 个源里**自动挑最快的**下载 Node 安装包：

1. https://nodejs.org/dist/ （Node.js 官方）
2. https://registry.npmmirror.com/-/binary/node/ （阿里 / 淘宝镜像，国内最快）
3. https://mirrors.tencent.com/nodejs-release/ （腾讯云镜像）
4. https://mirrors.huaweicloud.com/nodejs/ （华为云镜像）

按你的 Mac 架构挑对应包：

- Apple Silicon (M1/M2/M3/M4) → `node-v20.x.x-darwin-arm64.tar.xz`
- Intel Mac → `node-v20.x.x-darwin-x64.tar.xz`

下载完毕会**自动核对 SHA256 哈希值**（防止下载到被篡改的包）。

下载完成后解压到 `~/Library/Application Support/cc-switch/runtime/node/`，整个过程 1-2 分钟（取决于网速）。

### 4.4 装失败回滚

如果下载失败 / 解压失败 / 校验失败，CC Launcher 会：

1. 杀掉所有可能在跑的 node 进程
2. 删掉 `~/Library/Application Support/cc-switch/runtime/node/` 整个目录
3. 弹"安装失败 + 重试 / 手动打开官网"按钮
4. 在 `~/.cc-switch/install.log` 写一条失败记录（方便排障）

---

## 5. CLI 安装（npm + 智能选源）

私有 Node runtime 就位后，CC Launcher 会自动跑两条 npm 安装命令把 Claude Code 和 Codex CLI 装上。

### 5.1 装 Claude Code

包名（npm 上的官方包）：`@anthropic-ai/claude-code@latest`
当前最新版本：`2.1.148`（2026-05-22 核验）
要求的 Node 版本：≥ 18（CC Launcher 用 Node 20）
平台二进制：通过 npm 的 `optionalDependencies` 机制按你的架构自动拉对应的 `@anthropic-ai/claude-code-darwin-arm64` 或 `-darwin-x64` 子包

### 5.2 装 Codex CLI

包名：`@openai/codex@latest`
当前最新版本：`0.133.0`（2026-05-22 核验）
要求的 Node 版本：≥ 16
平台二进制：通过 `optionalDependencies` 按架构拉 `@openai/codex-darwin-arm64` 或 `-darwin-x64`

### 5.3 registry 智能选源

**registry**（npm 的镜像服务器，类似软件商店）有 4 个候选，都是编译期硬编码的白名单：

| Registry | URL | 说明 |
|---|---|---|
| npm 官方 | https://registry.npmjs.org | 海外用户首选 |
| 阿里 / 淘宝 | https://registry.npmmirror.com | 国内首选 |
| 腾讯 | https://mirrors.tencent.com/npm | 国内备选 |
| 华为云 | https://mirrors.huaweicloud.com/repository/npm | 国内备选 |

启动时 CC Launcher 会**并行**向 4 个 registry 发一个真实的小包查询请求，**谁先返回 200 OK 谁就被选中**。结果缓存 24 小时（避免每次启动都跑 5 秒探测）。

### 5.4 装后校验（重要）

每装完一个 CLI，CC Launcher 都会跑：

```
~/.cc-switch/runtime/node_modules/.bin/claude --version
~/.cc-switch/runtime/node_modules/.bin/codex --version
```

如果版本号格式正确（形如 `2.1.148`），算作"安装成功"。如果超时（10 秒）或输出对不上，算作失败 → 自动回滚。

### 5.5 用户视角

整个过程你在 UI 上看到的：

- 进度条（百分比 + 当前阶段）
- 状态文字：「正在挑选最快的软件源…」→「下载中… 30%」→「校验中…」→「完成」
- 失败时弹红色错误卡片 + "查看详情"链接

### 5.6 失败时的 6 步回滚

万一某一步失败，CC Launcher 自动按倒序撤销：

1. 杀掉所有可能正在跑的该 CLI 子进程
2. 删除 `~/.cc-switch/runtime/node_modules/@anthropic-ai/claude-code` 或 `@openai/codex` 整个子树
3. 删除 `.bin/` 里的 `claude` / `codex` 软链
4. 清空可能残留的空目录
5. 不触碰你系统的 PATH / shell rc / Launchd 配置（CC Launcher 从来不动这些）
6. 在 `~/.cc-switch/install.log` 写一条 `type=rollback` 记录，UI 弹"安装失败，已自动还原"

回滚完毕后用户可以放心点"重试"，不会有残留状态污染下次安装。

---

## 6. Profile 创建（一键启动前必做）

**Profile**（配置档案，类似游戏存档）是 CC Launcher 的核心概念：一个 Profile 把 "用哪个 CLI + 用哪个 Provider + 装了哪些 MCP / Skills + 默认设置" 打包成一个可切换单元。

> Profile 概念与 Windows 版完全一致。详细概念说明可见 [env-setup-windows-zh.md 第 6 节](./env-setup-windows-zh.md#6-profile-创建一键启动前必做)。

### 6.1 新建 Profile

首次启动 Onboarding 流程会引导你建第一个 Profile：

1. **取个名字** —— 比如 `我的写代码`
2. **选 CLI** —— Claude Code 或 Codex CLI 二选一（MVP 阶段只支持这两个）
3. **选 Provider**（API 服务商） —— 官方 Anthropic / OpenAI，或者第三方代理（详见 [proxy-guide-zh.md](./proxy-guide-zh.md)）

> 📸 截图：新建 Profile 三步表单

### 6.2 自动建工作目录

每建一个 Profile，CC Launcher 自动在以下位置创建对应的工作目录：

```
~/cc-launcher-projects/<profile-id>/
```

CLI 启动后**只能在这个目录里读写文件**（沙盒强制锁定，详见第 7 节）。

### 6.3 系统终端选择

CC Launcher 不内置终端，启动 CLI 时会自动调用系统终端：

1. **首选 Terminal.app**（macOS 自带）— 通过 `osascript` 命令 `tell application "Terminal" to do script "..."` 拉起一个新窗口
2. **回退** iTerm2 — 如果检测到你装了 iTerm2 也可以用，但 MVP 默认只用 Terminal.app

启动后系统终端窗口归你管，CC Launcher 不再干预 stdin（输入）。

---

## 7. 沙盒生效说明（小白安抚版）

### 7.1 双层禁令是什么

CC Launcher 的沙盒分两层：

| 层级 | 名称 | 能不能解锁 | 实现位置 |
|---|---|---|---|
| **L1** | 软拦截层 | 可以（专家模式 + 二次确认 + 输入规则名） | 配置文件（CLI 自带的 `permissions.deny` 等） |
| **L2** | 硬红线层 | **永远不能**（编译进二进制，没有任何按钮 / 设置 / API 能关闭它） | CC Launcher 源代码 |

### 7.2 L1 软拦截示例（你能见到的弹窗）

| 场景 | 弹窗内容 |
|---|---|
| 让 AI 跑 `sudo` 提权命令 | "尝试提权运行命令，未启用专家模式时禁止" |
| 让 AI 在工作目录外写文件 | "重定向写入工作目录外的路径" |
| 让 AI `curl xxx \| sh` 直接管道执行远程脚本 | "curl 直接管道到 shell，存在恶意脚本执行风险" |
| 让 AI 加 `--dangerously-skip-permissions`（Claude Code 跳过权限确认的危险标志） | "Claude Code 危险标志：跳过权限确认"（**这条不允许解锁**） |

### 7.3 L2 硬红线示例（永远不让你做的事，举具体例子）

不管你怎么解锁、怎么改设置、怎么诱导 AI，下面这些 CC Launcher 都**直接拒绝执行 + 写审计日志**：

| 类别 | 具体命令 / 行为 |
|---|---|
| **清盘** | `rm -rf /`、`rm -rf ~`、`rm -rf /Users/<你>`、`mkfs.*`、`dd of=/dev/disk*`、`chmod -R 000 /` |
| **改启动项** | 改 `/Library/LaunchDaemons/`、改 `/System/Library/LaunchDaemons/`、改 systemd 关键单元（在 Linux/WSL 上） |
| **改 hosts** | 改 `/etc/hosts`、改 `/private/etc/hosts`（macOS 上 `/etc/hosts` 本质是软链到 `/private/etc/hosts`） |
| **改 Launcher 自身** | 写 `~/.cc-switch/` 下任何文件（除非通过 CC Launcher GUI） |
| **反弹 shell**（远程控制后门） | `bash -i >& /dev/tcp/`、`nc -lp ... -e /bin/sh`、`python -c "...socket...connect..."` 等模式 |
| **已知勒索特征** | `openssl enc ... -out /` 整盘加密、`bitcoin.dat encrypt` 等签名 |

每命中一条 L2 都会被立刻拦截，并在 `~/.cc-switch/audit.log` 写一行 JSON 记录（带时间戳、规则 id、触发的命令片段）。

### 7.4 macOS 沙盒实现：sandbox-exec + SBPL

macOS 上 CC Launcher 启动 CLI 时实际跑的是这样一条命令链：

```
/usr/bin/sandbox-exec -p "<策略文本>" -DWRITABLE_ROOT_0=~/cc-launcher-projects/<profile-id> -- <CLI 绝对路径> <参数>
```

**`sandbox-exec`**：苹果自带的命令行沙盒工具，从 macOS 10.5 起内置至今。虽然 man page 标 `DEPRECATED`，但 Chrome、Firefox、Codex CLI、Claude Code 等都在用，**短期内不会被移除**。

**策略文本**用 SBPL（Sandbox Profile Language，苹果自创的策略描述语言，基于 TinyScheme）写，关键规则：

- **默认全部拒绝** —— `(deny default)`
- **允许在工作目录读写** —— `(allow file-write* (subpath "~/cc-launcher-projects/<profile-id>"))`
- **明确拒绝即使在白名单内的子路径** —— `(deny file-write* (literal "/etc/hosts"))`
- **TTY 必须放行** —— Claude Code / Codex 都是交互式 TUI，依赖 `pseudo-tty`，策略里包含 `(allow pseudo-tty)` 和 `/dev/ptmx` 读写规则
- **网络默认拒绝**，按需放行 `(allow network*)`

这套策略**在 spawn 之前完成构造**，编译进 CC Launcher 二进制，CLI 启动后没有任何路径能从沙盒内解除策略。

### 7.5 ⚠️ Claude Code macOS 上的内置 sandbox

**好消息**：macOS 上 Claude Code **自带**一套基于 Apple Seatbelt 的内置 sandbox（开关在配置文件里：`sandbox.enabled = true`）。

CC Launcher 给 Claude Code 注入的 settings 默认就开了这一项，**等于双层 sandbox**：

1. 外层：CC Launcher 用 `sandbox-exec` 包裹整个 CLI 进程
2. 内层：Claude Code 自己再用 `sandbox.enabled` 限制 Bash 子进程的网络 / 文件访问

两层叠加比 Windows 单层（Job Object）安全等级高一档。这是为啥团队推荐"小白用 macOS 时主用 Claude Code"。

### 7.6 ⚠️ MCP 子进程独立沙盒说明

**MCP**（Model Context Protocol，模型上下文协议，可以理解为 AI 工具的"插件市场")的服务器进程**独立于 Claude / Codex 主进程运行**。

- MCP 子进程的权限**不受 Claude Code / Codex 的 `permissions` 模型约束**
- 因此 CC Launcher 把 MCP 子进程**也用 `sandbox-exec` 单独包**，让它继承同样的工作目录限制和 L2 红线
- 这是为什么"必须经 CC Launcher 启动"，不能用其他 GUI 启动 MCP

### 7.7 Windows 用户对应章节

Windows 的沙盒实现走 Job Object + Restricted Token，且 Claude Code **在 Windows 上没有内置 sandbox**，所以风险等级和实现细节都不一样。见 [env-setup-windows-zh.md 第 7 节](./env-setup-windows-zh.md#7-沙盒生效说明小白安抚版)。

---

## 8. 已知坑（macOS 特有，必读）

### 8.1 Gatekeeper / com.apple.quarantine 处理（"无法验证开发者" 弹窗）

**现象**：首次启动 CC Launcher.app 时被 Gatekeeper 拦截，弹"无法验证开发者"。

**怎么办**：

- **正常路径**（推荐）：见 2.2 节第 3 步，去"系统设置 → 隐私与安全性"点"仍要打开"
- **极端情况**：如果"仍要打开"按钮也找不到，在终端跑一次：
  ```
  xattr -d com.apple.quarantine /Applications/CC\ Launcher.app
  ```
  这一行命令的意思是"移除苹果给这个 App 打的'下载隔离'标记"。
- CC Launcher 团队会做苹果开发者签名 + 公证（notarize），正式 release 后不应该再有这个弹窗。如果还出现，可能是你下载的是旧版本或来源不正规。

### 8.2 Apple Silicon 上 Rosetta 自动检测 + 安装

**现象**：你是 M1/M2/M3/M4 Mac，但 CC Launcher 提示"需要安装 Rosetta"。

**正解**：

- MVP 阶段 CC Launcher **默认下载 arm64 原生 Node**，**通常不需要 Rosetta**
- 极少数情况下（比如某个 npm 包没有 arm64 二进制只有 x64），需要 Rosetta 把 x64 翻译成 arm64 跑。这时跑：
  ```
  softwareupdate --install-rosetta --agree-to-license
  ```
- 命令本身是苹果官方提供的，安全。装完后无需重启 Mac

### 8.3 xcode-select 装 Git

**现象**：环境探测显示"Git 缺失"。

**正解**：

CC Launcher 弹"一键装 Git"按钮，背后实际跑的是：

```
xcode-select --install
```

这会触发一个**苹果系统对话框**问你"是否安装 Command Line Tools for Xcode"。点"安装"，然后等几分钟。

装完后你会得到 Git、clang、make、ssh 等一大堆基础开发工具。**不需要装完整的 Xcode**（那要 10 GB+，CLI tools 只要 1-2 GB）。

### 8.4 macOS 12 / 13 / 14 路径差异

不同 macOS 版本里有些细微路径差异，CC Launcher 已经处理好，了解一下即可：

| 项目 | macOS 12 Monterey | macOS 13 Ventura | macOS 14 Sonoma | macOS 15 Sequoia |
|---|---|---|---|---|
| 系统设置入口 | "系统偏好设置" | "系统设置" | "系统设置" | "系统设置" |
| Gatekeeper 路径 | 安全性与隐私 → 通用 | 隐私与安全性 | 隐私与安全性 | 隐私与安全性 |
| `sandbox-exec` 行为 | 完全支持 | 完全支持 | 完全支持 | 完全支持，man page 持续标 DEPRECATED |
| Apple Silicon Rosetta | 需要时手动装 | 同左 | 同左 | 同左 |

如果你的 macOS 版本未来到了 macOS 26+，**`sandbox-exec` 可能会有变化**，到时候 CC Launcher 会跟进发布新版本适配，请保持 Launcher 自身更新。

### 8.5 `~/.codex/config.toml` 跨平台路径不一致提示

**信息性提示**（仅供参考）：

- macOS / Linux：`~/.codex/config.toml`
- Windows：`~/.codex/config.toml`（一致）

但 Codex 的"管理员级"配置文件 `managed_config.toml` 在 macOS 上是 `/etc/codex/managed_config.toml`，在 Windows 上是 `~/.codex/managed_config.toml`。CC Launcher 已经按平台 cfg 分支处理，你不需要操心。

提到这点是因为：如果你的 IT 部门给你的 Mac 推过 MDM（移动设备管理）策略，可能会通过 `com.openai.codex` 域强制下发某些 Codex 设置，这些会优先于 CC Launcher 的配置生效。这不是 CC Launcher 的 bug，是设计上的优先级。

### 8.6 用户 zsh / bash rc 里的 ANTHROPIC_API_KEY 干扰

**现象**：你之前在 `~/.zshrc` 或 `~/.bash_profile` 里手动 `export ANTHROPIC_API_KEY=...`，启动 CC Launcher 后这个 key 优先级**高于** CC Launcher Profile 里配的 Provider。

**怎么办**：

- CC Launcher 在 3.1 节"环境变量冲突"探测时会扫到这一项，弹"一键清理 + 自动备份"按钮
- 备份会写到 `~/.cc-switch/backups/env-backup-<时间戳>.json`，随时可回滚
- 如果你确实想保留 shell rc 里的 export，可以在 CC Launcher 设置里把这个变量加入"忽略列表"

---

## 9. 卸载流程（必须给）

### 9.1 卸载 CC Launcher 本体

把 `/Applications/CC Launcher.app` 拖到废纸篓，然后清空废纸篓。

> 📸 截图：访达 → 应用程序 → 右键 CC Launcher.app → 移到废纸篓

### 9.2 清理私有 Node runtime 和 CLI

CC Launcher 自带卸载不会删用户数据。如果想彻底干净，再手动删两个目录：

```
~/Library/Application Support/cc-switch/
~/.cc-switch/
```

第一个目录存私有 Node + node_modules（约 200 MB）。
第二个目录存配置、日志、备份（约 5-50 MB，看你用了多久）。

在终端跑：

```
rm -rf ~/Library/Application\ Support/cc-switch
rm -rf ~/.cc-switch
```

### 9.3 清理 Profile 数据

```
~/cc-launcher-projects/
```

这里是所有 Profile 的工作目录。**先备份你的代码再删！**

```
rm -rf ~/cc-launcher-projects
```

### 9.4 撤销环境变量修改

**MVP 阶段 CC Launcher 默认不修改你的 shell 配置文件**（不写 `~/.zshrc` / `~/.bash_profile`）。所以正常情况下没东西需要撤销。

如果你启用过"全局 PATH 注入（高级模式）"：

- CC Launcher 在 `~/.zshrc` 里追加了一段用 `# >>> cc-switch managed BEGIN >>>` / `# <<< cc-switch managed END <<<` 标记包裹的 export 语句
- 卸载向导会问你"是否撤销 PATH 注入"，点是即可（CC Launcher 会用正则定位 BEGIN/END 标记并删除整段）

如果你之前用 CC Launcher 的"环境变量冲突清理"删过 `ANTHROPIC_*` / `OPENAI_*` 等冲突变量：

- 备份在 `~/.cc-switch/backups/env-backup-<时间戳>.json`
- 在 CC Launcher 卸载前可在 UI 里点"还原所有环境变量"恢复

---

## 10. FAQ（小白真问题）

### Q1：我把 API key 写哪里？

**A**：完全不需要碰文件。CC Launcher 的 UI 里有"Provider 管理"页面，把 API key 粘进对应的输入框就好。CC Launcher 内部加密存到 SQLite（`~/.cc-switch/cc-switch.db`），不会写到 `~/.zshrc` 也不会写到 `~/.bash_profile`。

### Q2：启动 CLI 后命令在哪个窗口跑？

**A**：CC Launcher 会自动拉起 Terminal.app，一个新窗口弹出来，Claude Code / Codex CLI 就在那里跑。这是**系统终端**，归你管。CC Launcher 本身的窗口可以最小化不影响。

### Q3：切 Profile 后 CLI 要重启吗？

**A**：要。当前正在跑的 CLI 进程**不会热切换**。切完 Profile 后关掉旧终端窗口，再在 CC Launcher 点"一键启动"。

### Q4：装失败怎么彻底清干净重来？

**A**：

1. 在 CC Launcher 点"卸载并清理"
2. 如果 CC Launcher 自身打不开了：
   - `rm -rf ~/Library/Application\ Support/cc-switch` （删 Node + CLI）
   - `rm -rf ~/.cc-switch` （删配置 + 日志）
   - 然后重新挂载 .dmg 安装包

### Q5：我手敲了 `rm -rf /` 真的不会清盘吗？

**A**：

- 如果你在 CC Launcher 拉起的终端里手敲 `rm -rf /`，命令会被**传给 CLI 主进程**，由 CLI 自己处理（不在 CC Launcher 视线内）
- 但是 **L2 硬红线是命令字符串级别的预拦截**，命中 `rm -rf /` 这种字符串模式的 spawn 请求会被 Launcher 直接拒绝
- 退一万步说，即便绕过了字符串拦截，**`sandbox-exec` 策略明确拒绝 `/etc/`、`/System/`、`/Library/LaunchDaemons/` 等所有系统路径的写入**，CLI 进程没有任何方式逃出策略
- 真正能清盘的命令（`diskutil eraseDisk`, `mkfs`, `dd of=/dev/disk*`）需要 root 权限，CC Launcher 默认以普通用户启动，根本拿不到 sudo 凭据

简而言之：在 CC Launcher 默认配置下，**清盘命令在三层防护下都跑不通**（命令拦截 → sandbox-exec 策略 → 缺 sudo 权限）。

### Q6：我能用代理 / VPN 吗？

**A**：可以。CC Launcher 自动读 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量，也会通过 `scutil --proxy` 读 macOS 系统代理设置（"系统设置 → 网络 → 详细信息 → 代理"），并透传给 npm。也可以在 Settings 里手动配置代理服务器。详见 [proxy-guide-zh.md](./proxy-guide-zh.md)。

### Q7：装完后能离线用吗？

**A**：装完之后，CC Launcher 自己能离线启动（UI 能打开）。但跑 Claude Code / Codex CLI 时它们会**调用云端 API**，那部分必须联网。"离线场景下不崩溃"指的是 CC Launcher 启动时网络不通也不会卡死，但实际跑 AI 需要网络。

### Q8：CC Launcher 会自动更新吗？

**A**：CC Launcher 自身会有版本检查，但**默认禁用 Claude Code / Codex CLI 的自动更新**（通过设置 `DISABLE_AUTOUPDATER=1` 环境变量），原因是避免 CLI 自动升级后行为漂移让你出错。CC Launcher 升级 CLI 时会用同样的智能选源 + 装后校验 + 失败回滚流程。

### Q9：我能同时跑 Claude Code 和 Codex CLI 吗？

**A**：可以。两个 CLI 进程互不影响（在不同终端窗口里）。CC Launcher 也支持给它们配不同的 Profile（不同的工作目录、不同的 MCP 集）。

### Q10：装完之后能在普通 Terminal 里直接敲 `claude` 启动吗？

**A**：MVP 默认**不行**。CC Launcher 不会动你的 `~/.zshrc` / `~/.bash_profile`（这是设计选择，避免污染 shell 环境）。如果你确实需要，可以在"专家模式 → 启用全局 PATH 注入"打开。打开后会在 `~/.zshrc` 里追加一段带 BEGIN/END 标记的 export 语句。打开后请重启 Terminal 窗口或 `source ~/.zshrc`，新 PATH 才会生效。

---

## 11. References

- 上游 cc-switch 文档：[https://github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)
- Claude Code 官方文档：[https://code.claude.com/docs](https://code.claude.com/docs)
- Codex CLI 官方文档：[https://developers.openai.com/codex](https://developers.openai.com/codex)
- Node.js 官方下载：[https://nodejs.org/zh-cn/download/](https://nodejs.org/zh-cn/download/)
- Xcode Command Line Tools 安装：执行 `xcode-select --install`，或访问 [https://developer.apple.com/xcode/resources/](https://developer.apple.com/xcode/resources/)
- 4 个 npm 镜像：
  - 官方：https://registry.npmjs.org
  - 阿里 / 淘宝：https://registry.npmmirror.com
  - 腾讯：https://mirrors.tencent.com/npm
  - 华为云：https://mirrors.huaweicloud.com/repository/npm
- macOS sandbox-exec 资料：
  - Mojave manpage（历史参考）：[https://www.unix.com/man-page/mojave/1/sandbox-exec/](https://www.unix.com/man-page/mojave/1/sandbox-exec/)
  - Chromium macOS sandbox 政策（CC Launcher 策略原型参考）：[https://source.chromium.org/chromium/chromium/src/+/main:sandbox/policy/mac/common.sb](https://source.chromium.org/chromium/chromium/src/+/main:sandbox/policy/mac/common.sb)
- 相关姊妹文档：[env-setup-windows-zh.md](./env-setup-windows-zh.md)
