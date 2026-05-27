# CC Launcher 环境配置完整路线（Windows）

> 这是一份"完全不懂电脑也能照做"的 Windows 路线书。
> macOS 用户请看：[env-setup-macos-zh.md](./env-setup-macos-zh.md)

---

## 0. 目标受众

这份文档写给：

- 完全小白：**不懂 AI / 不懂编程 / 不懂"什么叫终端"**
- 只想点一个图标就能用上 Claude Code（Anthropic 出的 AI 编程助手）和 Codex CLI（OpenAI 出的 AI 编程助手）
- 担心装完一堆软件会把电脑搞坏 / 怕被勒索病毒清盘

阅读本文不需要你提前知道任何技术名词。所有专业词都会在出现的时候用大白话括号解释。

---

## 1. 准备阶段（30 秒检查）

打开"设置 → 系统 → 关于"，对照下表看看你的电脑达不达标。

| 检查项 | 绿色（够用） | 黄色（能装但慢） | 红色（装不了） |
|---|---|---|---|
| **Windows 版本** | Windows 10 build 19041 或更高（2020 年 5 月以后的更新）、Windows 11 | Windows 10 旧 build（19041 之前） | Windows 7 / Windows 8 |
| **CPU 架构**（处理器类型） | x64（64 位 Intel/AMD）或 arm64（高通 Snapdragon） | 都行 | 32 位（极少见，10 年以上老机） |
| **CPU 核心数**（处理器核数） | 4 核及以上 | 2-3 核 | 1 核（基本买不到了） |
| **内存（RAM）** | 8 GB 及以上 | 4-8 GB | 不到 4 GB |
| **磁盘剩余空间**（C 盘或用户文件夹所在盘） | 5 GB 以上 | 2-5 GB | 不到 2 GB |
| **网络** | 能打开 [百度](https://www.baidu.com) 和 [GitHub](https://github.com) 任一个 | 只能打开百度（国内镜像够用） | 完全无网 |

> 📸 截图：Windows 设置 → 系统 → 关于 页面，红框框出"Windows 规格"和"系统类型"两行

**怎么看磁盘剩余空间**：打开"此电脑"，看 C 盘下面那条进度条剩多少 GB。

**怎么看内存**：同样在"此电脑"右键空白 → 属性 → 已安装内存。

如果你哪一项是红色，先解决它再继续。比如磁盘不够就清理 C 盘的下载、回收站、旧文档。

---

## 2. CC Launcher 安装

### 2.1 下载

打开 CC Launcher 的 Releases 页面（项目发布页面），找到最新版本，下载文件名形如：

```
cc-launcher_<版本号>_x64_zh-CN.msi      （64 位 Intel/AMD 用）
cc-launcher_<版本号>_arm64_zh-CN.msi    （Snapdragon ARM 用，极少数人需要）
```

> Releases 页面会随项目发布更新，请到项目首页查看最新地址。

`.msi` 是 Windows 的标准安装包格式，双击即可安装。

> 📸 截图：浏览器下载完成后，资源管理器里看到 `.msi` 文件的样子

### 2.2 安装步骤

1. **双击下载好的 .msi 文件**
2. **首次启动会弹出"Windows Defender SmartScreen"蓝色窗口**（"未识别的发布者"或"不常见的应用"）
   - 这是 Windows 对新软件的默认警告，不代表病毒
   - 点击"**更多信息**"，然后点击"**仍要运行**"
3. **不需要点"以管理员身份运行"**。CC Launcher 默认装到用户目录，不需要管理员权限
4. **安装向导一路点"下一步"**。默认路径是 `C:\Users\<你的用户名>\AppData\Local\Programs\cc-launcher\`
5. **安装完成后会自动在桌面 / 开始菜单生成快捷方式**

> 📸 截图：SmartScreen 蓝色弹窗 + "更多信息 / 仍要运行"两个按钮位置

### 2.3 首次启动 Onboarding（引导）

第一次双击 CC Launcher 图标启动后，会出现：

1. **欢迎页** — 一段中文说明"我们是谁、我们能做什么"
2. **环境自检页** — 后台自动跑探测（详见第 3 节），不需要你做任何事
3. **安全提示页** — 告诉你"默认沙盒严格，不会清盘"
4. **首个 Profile 创建引导** — 走到第 6 节再细看

### 2.4 安全提示

CC Launcher 默认开启**双层沙盒**（详见第 7 节）：

- **不会清盘** — 即使 AI 自作主张敲 `format C:` / `rm -rf /` 这种危险命令，硬红线层会直接拒绝
- **不会改你的系统文件** — Hosts 文件 / 注册表启动项 / 系统目录都被锁死
- **不会偷偷修改其他位置** — CLI 只能在 `C:\Users\<你的用户名>\cc-launcher-projects\` 内动手

唯一会"动你电脑"的操作是：

- 在 `C:\Users\<你的用户名>\AppData\Local\cc-switch\` 下安装一份 Node.js 私有运行时
- 在 `C:\Users\<你的用户名>\.cc-switch\` 下存放配置和日志

这两个目录**全部位于你的用户文件夹下**，不需要管理员权限，也不会污染 Program Files 或 Windows 目录。

---

## 3. 自动环境探测（CC Launcher 自动跑）

CC Launcher 启动后会用 1-3 秒跑完 **17 项探测**。每一项都用绿 / 黄 / 红三色标记。下面按重要性分组解释。

### 3.1 必需项（红色会阻塞启动）

| 探测项 | 探测什么 | 绿色含义 | 黄色含义 | 红色会怎样 |
|---|---|---|---|---|
| **Node.js 可执行**（node 命令） | 系统里有没有 Node（npm 的运行环境，类似 Python 之于 pip） | 系统已装 Node 20 或更高 | 装了 Node 18-19（能用，但 CC Launcher 会用私有 20） | 没装 → **CC Launcher 自动给你装一份私有的，不动系统**（详见第 4 节） |
| **npm 可执行**（包管理工具） | npm 命令在不在 | npm 10 及以上 | npm 9 | 没装 → 跟着 Node 一起装 |
| **Git 可执行**（代码版本管理工具） | Git 在不在 | 任何现代版本 | — | 没装 → **CC Launcher 弹"一键装 Git"按钮**，会下载 `Git-2.54.0-64-bit.exe` 并以静默模式安装 |
| **磁盘剩余空间** | C 盘（或用户文件夹所在盘）还剩多少 | 大于 5 GB | 2-5 GB | 不到 2 GB → 不允许装东西，弹"打开此电脑清理"按钮 |
| **网络可达性** | 能不能连上 npm 软件源（4 个候选服务器，详见第 5 节） | 至少 1 个延迟低于 1 秒 | 全都通但都大于 1 秒 | 4 个全连不上 → "网络不通"提示，所有联网按钮变灰 |
| **环境变量冲突** | 你之前装过其他 AI 工具留下的 `ANTHROPIC_*` / `OPENAI_*` / `GEMINI_*` 变量 | 没有冲突 | — | 有冲突 → 弹"一键清理 + 自动备份"按钮 |
| **工作目录可写性** | `C:\Users\<你>\cc-launcher-projects\` 能不能创建/写入 | 可写 | — | 不可写 → 自动尝试创建，失败则提示权限问题 |

### 3.2 建议项（黄色提醒，不阻塞）

| 探测项 | 探测什么 | 绿色 | 黄色 | 红色 |
|---|---|---|---|---|
| **OS 版本** | Windows 是哪个 build | Win 10 build 19041+ / Win 11 | Win 10 旧 build | Win 7-8 |
| **CPU 核心数** | 物理核数 | 大于等于 4 | 2-3 | 1 |
| **总内存** | 装机内存 | 大于等于 8 GB | 4-8 GB | 不到 4 GB |
| **可用内存** | 当前剩余 RAM | 大于 2 GB | 1-2 GB | 不到 1 GB（提示关掉其他程序） |
| **PATH 完整性** | 系统 PATH 里有没有 CC Launcher 需要的关键目录 | 都在 | 部分缺失 | 关键缺失 → Launcher 自动在子进程内注入，不动你的系统设置 |

### 3.3 信息项（Windows 特有）

下面这些是 Windows 系统级权限相关因素。MVP 阶段**只提示不自动改**（因为修改它们风险大、需要管理员权限）。

| 探测项 | 探测什么 | 出现警告时 CC Launcher 会怎样 |
|---|---|---|
| **管理员状态** | 你是不是以"管理员身份"启动了 Launcher | 弹"无需以管理员运行"提示，建议你用普通账户。原因：管理员模式下沙盒会被绕过 |
| **PowerShell 执行策略** | `Get-ExecutionPolicy` 返回啥 | 如果是 `Restricted`（最严格），某些 npm 脚本会失败。弹"复制这段命令到 PowerShell 跑一下" + 解释 |
| **Windows Defender 排除项** | npm 安装目录在不在 Defender 实时扫描白名单里 | 仅信息显示。Defender 扫描会拖慢 npm install，但 **MVP 不会自动加排除项**（怕误操作） |
| **系统代理** | `HTTP_PROXY` / `HTTPS_PROXY` 环境变量是否设了代理 | 如果设了，自动透传给 npm。如果代理不可达，提示你检查 |

> 📸 截图：CC Launcher 主界面"系统自检"卡片，绿/黄/红分组样式

---

## 4. Node.js 私有 runtime 安装（Launcher 自动）

### 4.1 为什么用私有 runtime

如果你的电脑已经装了 Node.js（比如以前学过编程），CC Launcher 仍然**优先用自己的私有版本**。原因：

1. **不污染系统**：CC Launcher 卸载时一个 rmdir 就能干净清空，不留尾巴
2. **可控版本**：你升级系统 Node 不会让 CC Launcher 突然跑挂
3. **不需要管理员**：装到用户目录，不弹 UAC
4. **统一基线**：所有 CC Launcher 用户跑的都是同一份 Node 20 LTS，少踩兼容性的坑

### 4.2 落点路径（固定）

```
C:\Users\<你的用户名>\AppData\Local\cc-switch\runtime\node\
```

里面会有：

```
node.exe
npm.cmd
npx.cmd
node_modules\
... 等等
```

### 4.3 Node 20 LTS 安装包来源

CC Launcher 会从以下 4 个源里**自动挑最快的**下载 Node 安装包：

1. https://nodejs.org/dist/ （Node.js 官方）
2. https://registry.npmmirror.com/-/binary/node/ （阿里 / 淘宝镜像，国内最快）
3. https://mirrors.tencent.com/nodejs-release/ （腾讯云镜像）
4. https://mirrors.huaweicloud.com/nodejs/ （华为云镜像）

如果你在「安装源设置」里填了自建 VPS 的 `Node dist mirror`，CC Launcher 会先尝试你填的地址，再回退到这 4 个内置镜像。

下载完毕会**自动核对 SHA256 哈希值**（防止下载到被篡改的包）。

下载完成后解压到 `cc-switch\runtime\node\`，整个过程 1-2 分钟（取决于网速）。

### 4.4 装失败回滚

如果下载失败 / 解压失败 / 校验失败，CC Launcher 会：

1. 杀掉所有可能在跑的 node 进程
2. 删掉 `cc-switch\runtime\node\` 整个目录
3. 弹"安装失败 + 重试 / 手动打开官网"按钮
4. 在 `C:\Users\<你>\.cc-switch\install.log` 写一条失败记录（方便排障）

---

## 5. CLI 安装（npm + 智能选源）

私有 Node runtime 就位后，CC Launcher 会自动跑两条 npm 安装命令把 Claude Code 和 Codex CLI 装上。

### 5.1 装 Claude Code

包名（npm 上的官方包）：`@anthropic-ai/claude-code@2.1.150`
当前固定版本：`2.1.150`（2026-05-26 核验）
要求的 Node 版本：≥ 18（CC Launcher 用 Node 20）

### 5.2 装 Codex CLI

包名：`@openai/codex@0.133.0`
当前固定版本：`0.133.0`（2026-05-22 核验）
要求的 Node 版本：≥ 16

### 5.3 registry 智能选源

**registry**（npm 的镜像服务器，类似软件商店）默认有 4 个内置候选；如果你在设置里填了自建 VPS 的 `npm registry`，它会被优先插入到最前面，然后再回退到内置候选：

| Registry | URL | 说明 |
|---|---|---|
| npm 官方 | https://registry.npmjs.org | 海外用户首选 |
| 阿里 / 淘宝 | https://registry.npmmirror.com | 国内首选 |
| 腾讯 | https://mirrors.tencent.com/npm | 国内备选 |
| 华为云 | https://mirrors.huaweicloud.com/repository/npm | 国内备选 |

你的自建 VPS 可以直接提供同样格式的 npm registry、Node dist mirror 和 Git for Windows mirror。CC Launcher 会优先用你填的地址，失败后再回退到上面的内置源。

启动时 CC Launcher 会**并行**向 4 个 registry 发一个真实的小包查询请求，**谁先返回 200 OK 谁就被选中**。结果缓存 24 小时（避免每次启动都跑 5 秒探测）。

### 5.4 装后校验（重要）

每装完一个 CLI，CC Launcher 都会跑：

```
%LOCALAPPDATA%\cc-switch\runtime\claude\claude.cmd --version
%LOCALAPPDATA%\cc-switch\runtime\codex\codex.cmd --version
```

如果版本号格式正确（形如 `2.1.150`），算作"安装成功"。如果超时（10 秒）或输出对不上，算作失败 → 自动回滚。

### 5.5 用户视角

整个过程你在 UI 上看到的：

- 进度条（百分比 + 当前阶段）
- 状态文字：「正在挑选最快的软件源…」→「下载中… 30%」→「校验中…」→「完成」
- 失败时弹红色错误卡片 + "查看详情"链接

### 5.6 失败时的 6 步回滚

万一某一步失败，CC Launcher 自动按倒序撤销：

1. 杀掉所有可能正在跑的该 CLI 子进程
2. 删除 `%LOCALAPPDATA%\cc-switch\runtime\claude\` 或 `%LOCALAPPDATA%\cc-switch\runtime\codex\` 里的 npm prefix 内容
3. 删除对应 prefix 根目录里的 `claude.cmd` / `codex.cmd` shim
4. 清空可能残留的空目录
5. 不触碰你系统的 PATH / 注册表 / shell 配置（CC Launcher 从来不动这些）
6. 在 `~\.cc-switch\install.log` 写一条 `type=rollback` 记录，UI 弹"安装失败，已自动还原"

回滚完毕后用户可以放心点"重试"，不会有残留状态污染下次安装。

---

## 6. Profile 创建（一键启动前必做）

**Profile**（配置档案，类似游戏存档）是 CC Launcher 的核心概念：一个 Profile 把 "用哪个 CLI + 用哪个 Provider + 装了哪些 MCP / Skills + 默认设置" 打包成一个可切换单元。

### 6.1 新建 Profile

首次启动 Onboarding 流程会引导你建第一个 Profile：

1. **取个名字** —— 比如 `我的写代码`
2. **选 CLI** —— Claude Code 或 Codex CLI 二选一（MVP 阶段只支持这两个）
3. **选 Provider**（API 服务商） —— 官方 Anthropic / OpenAI，或者第三方代理（详见 [proxy-guide-zh.md](./proxy-guide-zh.md)）

> 📸 截图：新建 Profile 三步表单

### 6.2 自动建工作目录

每建一个 Profile，CC Launcher 自动在以下位置创建对应的工作目录：

```
C:\Users\<你的用户名>\cc-launcher-projects\<profile-id>\
```

CLI 启动后**只能在这个目录里读写文件**（沙盒强制锁定，详见第 7 节）。

### 6.3 系统终端选择

CC Launcher 不内置终端，启动 CLI 时会自动调用系统终端：

1. **首选 Windows Terminal**（`wt.exe`） — Windows 11 自带，Win 10 可从微软商店免费装
2. **回退** `cmd.exe` — Windows 自带，如果 wt.exe 不存在就用它

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
| 让 AI 跑 `sudo` / `runas` 提权命令 | "尝试提权运行命令，未启用专家模式时禁止" |
| 让 AI 在工作目录外写文件 | "重定向写入工作目录外的路径" |
| 让 AI `curl xxx \| sh` 直接管道执行远程脚本 | "curl 直接管道到 shell，存在恶意脚本执行风险" |
| 让 AI 加 `--dangerously-skip-permissions`（Claude Code 跳过权限确认的危险标志） | "Claude Code 危险标志：跳过权限确认"（**这条不允许解锁**） |

### 7.3 L2 硬红线示例（永远不让你做的事，举具体例子）

不管你怎么解锁、怎么改设置、怎么诱导 AI，下面这些 CC Launcher 都**直接拒绝执行 + 写审计日志**：

| 类别 | 具体命令 / 行为 |
|---|---|
| **清盘** | `rm -rf /`、`rm -rf ~`、`format C:`、`del C:\*`、`mkfs.*`、`dd of=/dev/sd*`、`chmod -R 000 /` |
| **改启动项** | 写 `HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Run`、写 `HKLM\SYSTEM\CurrentControlSet\Services`、改 systemd 关键单元 |
| **改 hosts** | 改 `C:\Windows\System32\drivers\etc\hosts`、改 `/etc/hosts` |
| **改 Launcher 自身** | 写 `C:\Users\<你>\.cc-switch\` 下任何文件（除非通过 CC Launcher GUI） |
| **反弹 shell**（远程控制后门） | `bash -i >& /dev/tcp/`、`nc -lp ... -e /bin/sh`、`powershell ... TcpClient` 模式等 |
| **已知勒索特征** | `openssl enc ... -out /` 整盘加密、`bitcoin.dat encrypt` 等签名 |

每命中一条 L2 都会被立刻拦截，并在 `~\.cc-switch\audit.log` 写一行 JSON 记录（带时间戳、规则 id、触发的命令片段）。

### 7.4 ⚠️ Windows 平台 Claude Code 特殊说明（重要）

**Claude Code 在 Windows 上没有内置的 OS 沙盒**（macOS / Linux / WSL2 上才有 `sandbox.enabled`）。

这意味着：

- **Windows 上的 L2 红线完全依赖 CC Launcher 自己的拦截层**（Job Object + 命令字符串规范化 + 静态正则规则集）
- Claude Code 自身只能靠 `permissions.deny` 规则做软拦截，**Anthropic 官方明确警告 Bash deny 规则容易被绕过**（命令参数变形、子 shell、重定向等）
- 因此**在 Windows 上使用 Claude Code 必须依赖 CC Launcher 启动**，不能直接在终端里敲 `claude` 命令绕开 Launcher

CC Launcher 通过以下机制兜底：

- Windows Job Object（把 CLI 子进程关进一个 Windows 内核级容器，限制 CPU / 内存 / 子进程数量 / 剪贴板 / 弹窗）
- 不允许 silent breakaway（子进程不能偷偷脱出 Job）
- 启动前对命令字符串做 L2 正则扫描（即使没法拦截 stdin，命令字符串里出现的危险模式会被 spawn 前预拦截）

### 7.5 ⚠️ MCP 子进程独立沙盒说明

**MCP**（Model Context Protocol，模型上下文协议，可以理解为 AI 工具的"插件市场")的服务器进程**独立于 Claude / Codex 主进程运行**。

- MCP 子进程的权限**不受 Claude Code / Codex 的 `permissions` 模型约束**
- 因此 CC Launcher 把 MCP 子进程**也关进同一个 Job Object**，让它和主 CLI 一样受 L2 红线保护
- 这是为什么"必须经 CC Launcher 启动"，不能用其他 GUI 启动 MCP

### 7.6 macOS 用户对应章节

macOS 的沙盒实现走 `sandbox-exec` + SBPL 策略语言，细节见 [env-setup-macos-zh.md 第 7 节](./env-setup-macos-zh.md#7-沙盒生效说明小白安抚版)。

---

## 8. 已知坑（Windows 特有，必读）

### 8.1 Windows Defender 实时扫描拖慢 npm install

**现象**：npm 装包过程中 Defender 会扫描每一个解压出来的 `.js` / `.node` 文件，安装时间从 30 秒变成 3 分钟。

**怎么办**：

- MVP 版本**不会自动给 Defender 加排除项**（因为修改 Defender 设置需要管理员权限 + 误改风险大）
- 如果你能接受手动操作 + 知道自己在做什么，可以在 PowerShell（**管理员模式**）里跑：
  ```
  Add-MpPreference -ExclusionPath "C:\Users\<你的用户名>\AppData\Local\cc-switch\runtime\node\node_modules"
  ```
- 如果你不知道自己在做什么，**就让它慢点跑**，等几分钟没关系

### 8.2 PowerShell 执行策略 = Restricted 时 npm 部分脚本失败

**现象**：npm postinstall 阶段某些 `.ps1` 脚本被 `Restricted` 策略拦截，报错 "无法加载文件，因为在此系统上禁止运行脚本"。

**怎么办**：

在 PowerShell（**当前用户作用域，不需要管理员**）里跑一次：

```
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

这个命令只放开"本地签名脚本"，安全性可控。改完后重启 CC Launcher 让它重新跑 npm install。

### 8.3 UAC 弹窗：默认装到用户目录其实不需要提权

**现象**：你看到 SmartScreen 弹窗就以为要点"是的，以管理员身份继续"。

**正解**：

- CC Launcher 安装目录是 `%LOCALAPPDATA%\Programs\cc-launcher\`（用户目录），**不需要管理员**
- Node runtime 装到 `%LOCALAPPDATA%\cc-switch\runtime\node\`，**也不需要管理员**
- 配置和日志在 `~\.cc-switch\`，**还是不需要管理员**
- 工作目录 `~\cc-launcher-projects\`，**仍然不需要管理员**

**唯一需要管理员的场景**：你主动选择"全局 PATH 注入（高级模式）"，让 `claude` 命令在普通 cmd 窗口里也能直接跑。MVP 默认**不启用**这个，所以正常使用全程不需要管理员。

### 8.4 Windows 平台 Claude Code 没有 OS sandbox

**这是 Claude Code 本身的事实**，不是 CC Launcher 的限制。详见第 7.4 节。在 Windows 上，**你必须信任 CC Launcher 的 Job Object 拦截层**作为主要安全屏障。

### 8.5 国内安全软件可能误报 Restricted Token API

**现象**：360 安全卫士 / 腾讯电脑管家 / 火绒等国内 AV 软件，看到 CC Launcher 调用 `CreateRestrictedToken` + `CreateProcessAsUserW` 这种"修改进程令牌"的 Windows API，会启发式扫描标记为可疑。

**正解**：

- 这些 API 是 Windows 官方提供的标准沙盒能力，**和 Chrome 的渲染进程沙盒、Edge 浏览器沙盒用的是同一套 API**
- 国内 AV 误报通常会随着 CC Launcher 发布版本累积"信誉积分"自动消失
- 实在被误删 / 被拦截，可以在 AV 软件的"信任区 / 白名单"里加入 CC Launcher 安装目录

### 8.6 `wt.exe` 不存在时回退 cmd.exe（旧 Win 10 老版本）

**现象**：你的 Windows 10 是 2018-2019 的老版本，没装 Windows Terminal。

**CC Launcher 行为**：自动回退用 `cmd.exe` 启动。功能完全一样，只是窗口外观朴素一点。

**建议**：免费从 Microsoft Store 装个 Windows Terminal，体验显著提升。

---

## 9. 卸载流程（必须给）

### 9.1 卸载 CC Launcher 本体

1. 打开"设置 → 应用 → 安装的应用"
2. 找到 "CC Launcher"，点"卸载"
3. 完成

> 📸 截图：Windows 设置中找到 CC Launcher 并卸载

### 9.2 清理私有 Node runtime 和 CLI

CC Launcher 自带卸载不会删用户数据。如果想彻底干净，再手动删两个目录：

```
C:\Users\<你的用户名>\AppData\Local\cc-switch\
C:\Users\<你的用户名>\.cc-switch\
```

第一个目录存私有 Node + node_modules（约 200 MB）。
第二个目录存配置、日志、备份（约 5-50 MB，看你用了多久）。

### 9.3 清理 Profile 数据

```
C:\Users\<你的用户名>\cc-launcher-projects\
```

这里是所有 Profile 的工作目录。**先备份你的代码再删！**

### 9.4 撤销环境变量修改

**MVP 阶段 CC Launcher 默认不修改你的系统 / 用户环境变量。** 所以正常情况下没东西需要撤销。

如果你启用过"全局 PATH 注入（高级模式）"：

- CC Launcher 在 `~\.cc-switch\backups\path-<时间戳>.txt` 保留了原 PATH 的完整副本
- 卸载向导会问你"是否撤销 PATH 注入"，点是即可

如果你之前用 CC Launcher 的"环境变量冲突清理"删过 `ANTHROPIC_*` / `OPENAI_*` 等冲突变量：

- 备份在 `~\.cc-switch\backups\env-backup-<时间戳>.json`
- 在 CC Launcher 卸载前可在 UI 里点"还原所有环境变量"恢复

---

## 10. FAQ（小白真问题）

### Q1：我把 API key 写哪里？

**A**：完全不需要碰文件。CC Launcher 的 UI 里有"Provider 管理"页面，把 API key 粘进对应的输入框就好。CC Launcher 内部加密存到 SQLite（`~\.cc-switch\cc-switch.db`），不会写到环境变量也不会写到 shell 配置。

### Q2：启动 CLI 后命令在哪个窗口跑？

**A**：CC Launcher 会自动拉起 Windows Terminal（或回退 cmd.exe），一个新窗口弹出来，Claude Code / Codex CLI 就在那里跑。这是**系统终端**，归你管。CC Launcher 本身的窗口可以最小化不影响。

### Q3：切 Profile 后 CLI 要重启吗？

**A**：要。当前正在跑的 CLI 进程**不会热切换**。切完 Profile 后关掉旧终端窗口，再在 CC Launcher 点"一键启动"。

### Q4：装失败怎么彻底清干净重来？

**A**：

1. 在 CC Launcher 点"卸载并清理"
2. 如果 CC Launcher 自身打不开了：
   - 删 `C:\Users\<你>\AppData\Local\cc-switch\runtime\` （删 Node + CLI）
   - 删 `C:\Users\<你>\.cc-switch\` （删配置 + 日志）
   - 然后重新运行 .msi 安装包

### Q5：我手敲了 `rm -rf /` 真的不会清盘吗？

**A**：

- 如果你在 CC Launcher 拉起的终端里手敲 `rm -rf /`，命令会被**传给 CLI 主进程**，由 CLI 自己处理（不在 CC Launcher 视线内）
- 但是 **L2 硬红线是命令字符串级别的预拦截**，命中 `rm -rf /` 这种字符串模式的 spawn 请求会被 Launcher 直接拒绝
- 即便绕过了字符串拦截（比如用变量拼接），Windows 上由于 NTFS 文件权限和 Defender 双重保护，普通用户身份**也无权删除 Windows 目录 / 其他用户的文件**
- 真正能清盘的命令（`format`, `mkfs`, `diskpart`）需要管理员权限，CC Launcher 默认以普通用户启动，根本拿不到这种权限

简而言之：在 CC Launcher 默认配置下，**清盘命令在三层防护下都跑不通**（命令拦截 → NTFS 权限 → 缺管理员令牌）。

### Q6：我能用代理 / VPN 吗？

**A**：可以。CC Launcher 自动读 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量并透传给 npm。也可以在 Settings 里手动配置代理服务器。详见 [proxy-guide-zh.md](./proxy-guide-zh.md)。

### Q7：装完后能离线用吗？

**A**：装完之后，CC Launcher 自己能离线启动（UI 能打开）。但跑 Claude Code / Codex CLI 时它们会**调用云端 API**，那部分必须联网。"离线场景下不崩溃"指的是 CC Launcher 启动时网络不通也不会卡死，但实际跑 AI 需要网络。

### Q8：CC Launcher 会自动更新吗？

**A**：CC Launcher 自身会有版本检查，但**默认禁用 Claude Code / Codex CLI 的自动更新**（通过设置 `DISABLE_AUTOUPDATER=1` 环境变量），原因是避免 CLI 自动升级后行为漂移让你出错。CC Launcher 升级 CLI 时会用同样的智能选源 + 装后校验 + 失败回滚流程。

### Q9：我能同时跑 Claude Code 和 Codex CLI 吗？

**A**：可以。两个 CLI 进程互不影响（在不同终端窗口里）。CC Launcher 也支持给它们配不同的 Profile（不同的工作目录、不同的 MCP 集）。

### Q10：装完之后能在普通 cmd 里直接敲 `claude` 启动吗？

**A**：MVP 默认**不行**。CC Launcher 不会把 `claude` / `codex` 加进你的系统 PATH（这是设计选择，避免污染用户环境）。如果你确实需要，可以在"专家模式 → 启用全局 PATH 注入"打开。打开后请重启所有终端窗口，新 PATH 才会生效。

---

## 11. References

- 上游 cc-switch 文档：[https://github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)
- Claude Code 官方文档：[https://code.claude.com/docs](https://code.claude.com/docs)
- Codex CLI 官方文档：[https://developers.openai.com/codex](https://developers.openai.com/codex)
- Node.js 官方下载：[https://nodejs.org/zh-cn/download/](https://nodejs.org/zh-cn/download/)
- Windows Terminal Microsoft Store：[https://aka.ms/terminal](https://aka.ms/terminal)
- Git for Windows：[https://git-scm.com/download/win](https://git-scm.com/download/win)
- 4 个内置 npm 镜像（另可在「安装源设置」里填自建 VPS 源，优先使用）：
  - 官方：https://registry.npmjs.org
  - 阿里 / 淘宝：https://registry.npmmirror.com
  - 腾讯：https://mirrors.tencent.com/npm
  - 华为云：https://mirrors.huaweicloud.com/repository/npm
- 相关姊妹文档：[env-setup-macos-zh.md](./env-setup-macos-zh.md)
