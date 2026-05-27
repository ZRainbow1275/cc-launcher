# GitHub Actions macOS 构建环境说明

> 这份说明用于把仓库里的 macOS 打包流程真正跑起来。
> 对应的工作流文件是 `.github/workflows/macos-build.yml`，发布流程还会复用 `.github/workflows/release.yml` 里的 macOS 规则。

## 两种构建模式

手动运行 `.github/workflows/macos-build.yml` 时有一个 `signing_mode` 选项：

- `unsigned-test`：默认值。不需要 Apple 证书、不需要 `macos-release` secrets，用于先产出 macOS 测试包验证功能链路。
- `signed-notarized`：正式签名 + 公证流程。需要下面的 `macos-release` Environment secrets。

`unsigned-test` 产物可以用于功能测试，但不是正式分发包。macOS 可能提示来源不明；测试时可以右键打开，或在确认来源可信后手动移除 quarantine 属性。正式给用户分发时必须使用 `signed-notarized`。

## 环境名

在 GitHub 仓库里创建一个 Environment，名称固定为：

```text
macos-release
```

如果你希望这个环境带审批门槛，可以在 GitHub 的 Environment 页面里额外配置 reviewers。

## 需要配置的 Secrets

只有运行 `signed-notarized` 时才需要把下面这些 secret 加到 `macos-release` 这个 Environment 里：

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `KEYCHAIN_PASSWORD`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `TAURI_SIGNING_PRIVATE_KEY`

可选项：

- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## 字段含义

- `APPLE_CERTIFICATE`：Apple Developer `.p12` 证书，通常以 base64 形式存储
- `APPLE_CERTIFICATE_PASSWORD`：导出 `.p12` 时设置的密码
- `KEYCHAIN_PASSWORD`：工作流里临时 keychain 的密码
- `APPLE_ID`：用于 notarization 的 Apple ID
- `APPLE_PASSWORD`：Apple 的 app-specific password
- `APPLE_TEAM_ID`：Apple Developer Team ID
- `TAURI_SIGNING_PRIVATE_KEY`：Tauri 签名私钥，工作流支持原始两行格式、base64 包裹格式，或单行 key body
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：如果你的私钥还有额外口令，就填这里；没有就留空

## 触发方式

- 手动构建 macOS 测试包：运行 `.github/workflows/macos-build.yml`，`signing_mode` 选择 `unsigned-test`
- 手动构建签名/公证包：运行 `.github/workflows/macos-build.yml`，`signing_mode` 选择 `signed-notarized`
- 打 tag 发布：走 `.github/workflows/release.yml`

## 产物命名

macOS 签名构建会产出以下文件名风格：

- `CC-Launcher-<version>-macOS.dmg`
- `CC-Launcher-<version>-macOS.zip`
- `CC-Launcher-<version>-macOS.tar.gz`

其中 `.dmg` 是推荐分发包，`.zip` 方便解压即用，`.tar.gz` 是 updater 用的更新产物。

macOS 无签名测试构建会产出以下文件名风格：

- `CC-Launcher-<version>-macOS-unsigned.dmg`
- `CC-Launcher-<version>-macOS-unsigned.zip`

无签名测试构建使用 `src-tauri/tauri.package.conf.json`，不会生成 updater 签名产物。

## 失败时先看什么

如果 `unsigned-test` 失败，优先检查：

1. `pnpm install --frozen-lockfile` 是否失败
2. macOS runner 是否能成功安装 Rust target
3. `pnpm tauri build --target universal-apple-darwin --config src-tauri/tauri.package.conf.json` 是否失败
4. 产物里是否生成了 `CC Launcher.app`

如果 `signed-notarized` 失败，优先检查：

1. `macos-release` Environment 是否真的有这些 secret
2. Apple 证书和签名私钥格式是否正确
3. Apple ID / app-specific password / Team ID 是否和当前开发者账号匹配
4. 证书是否过期、keychain 密码是否填错
