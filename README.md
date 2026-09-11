# InputMesh

[English](README.en.md) | 简体中文

InputMesh 是一个面向 Windows 与 macOS 的点对点键鼠共享应用。把应用运行在同一可信局域网内的电脑上，在画布中排列所有屏幕后，鼠标越过屏幕边缘即可切换到另一台电脑，键盘焦点也会一起切换。

项目当前为预览版。InputMesh 自有代码采用 [PolyForm Noncommercial License 1.0.0](LICENSE)，适合个人学习、研究、测试和其他非商业用途。

## 快速开始

### 1. 准备开发环境

- Node.js 24（仓库中的 `.nvmrc` 指定主版本）。
- pnpm 11.19.0（`package.json` 中的 `packageManager` 字段指定版本）。
- Rust stable，最低版本为 1.88。
- macOS 12 或更高版本：安装 Xcode Command Line Tools。
- Windows 10/11：安装带 MSVC C++ 工具的 Visual Studio Build Tools，并准备 WebView2 Runtime。

如果本机还没有 pnpm，可以使用 Corepack 或 npm 安装指定版本：

```bash
corepack enable
corepack prepare pnpm@11.19.0 --activate
```

也可以执行：

```bash
npm install --global pnpm@11.19.0
```

### 2. 克隆仓库

```bash
git clone https://github.com/Henry-L1/InputMesh.git
cd InputMesh
```

### 3. 安装依赖

在仓库根目录执行：

```bash
pnpm install --frozen-lockfile
```

这个命令会按照 `pnpm-lock.yaml` 安装前端、Tauri CLI 和测试依赖，并保持锁文件不变。

### 4. 启动开发版桌面应用

```bash
pnpm tauri dev
```

该命令会启动 React/Vite 前端和 Tauri/Rust 桌面后端。第一次运行还需要编译 Rust 依赖，可能比后续启动更慢。开发版可以验证真正的桌面能力，包括全局键鼠捕获、系统权限、局域网发现、加密连接和跨设备输入转发。

如果只想查看前端控制界面，可以运行：

```bash
pnpm dev
```

浏览器预览使用内置演示数据，不会捕获或注入真实键鼠，因此不能代替 `pnpm tauri dev` 的双机联调。

### 5. 配置系统权限

首次在 macOS 上启动时，请在“系统设置 → 隐私与安全性”中为 InputMesh 授予：

- 辅助功能
- 输入监控

macOS 15 及以上版本还可能需要允许本地网络访问。修改权限后请重启 InputMesh。

Windows 首次弹出防火墙提示时，只应在受信任的专用网络上允许访问。

### 6. 可选的 VS Code 扩展

推荐安装以下扩展，以获得 Rust、Tauri、TOML 和原生调试支持：

- [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

如果已安装 VS Code 的 `code` 命令，也可以执行：

```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension tauri-apps.tauri-vscode
code --install-extension tamasfe.even-better-toml
code --install-extension vadimcn.vscode-lldb
```

## 两台电脑联调

1. 将 Windows 和 macOS 电脑连接到同一个可信局域网，并分别启动 InputMesh。
2. 在双方界面核对 6 位配对码，确认一致后分别点击“允许”。
3. 在屏幕画布中拖动显示器，使相邻边与真实摆放一致；关闭不希望纳管的屏幕。
4. 点击“开始共享”，再把鼠标推过配置好的相邻边缘。

默认 TCP 监听端口从 `42424` 开始自动选择。发现不等于信任：只有用户确认过配对码的设备才能接收或注入输入。请不要在不受信任的公共 Wi-Fi 上启用键鼠共享。

## 验证、测试与打包

运行前端单元测试：

```bash
pnpm test
```

运行 TypeScript/Vite 生产构建：

```bash
pnpm build
```

运行 Rust 测试：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

一次运行常用检查：

```bash
pnpm check
```

`pnpm check` 会依次运行前端测试、前端构建和 Rust 测试。构建当前平台的安装包：

```bash
pnpm tauri build
```

Windows 和 macOS 安装包应分别在对应系统或 CI runner 上构建。当前预览包尚未进行 Apple Developer ID 公证或 Windows Authenticode 签名，正式稳定发布前仍需完成平台签名、公证和更完整的实机回归。

## 项目目标与架构

第一版目标包括：

- Windows 10/11 与 macOS 12+ 使用同一套 Tauri 2 / Rust 工程。
- 用 mDNS 自动发现局域网内的 InputMesh 实例。
- 用 Noise XX 握手建立端到端加密连接，并以双方相同的 6 位数字完成首次配对确认。
- 自动枚举每台电脑的显示器，支持拖动排列和逐屏启用/停用。
- 鼠标跨屏后转发鼠标、滚轮和键盘事件；键盘按物理键码转发，默认遵循 Windows 风格布局，不同步输入法布局。
- 任一电脑上的本地物理输入都能按“最近操作者优先”接管控制。
- `Ctrl + Alt + Esc` 是本地紧急释放快捷键。

```text
React 控制界面
      │ Tauri commands / events
Rust 应用运行时
      ├── 屏幕拓扑与配置持久化
      ├── mDNS 局域网发现
      ├── Noise XX 加密的 TCP 会话
      └── 平台输入层
            ├── macOS: CoreGraphics event tap（辅助功能 + 输入监控）
            └── Windows: low-level hooks / SendInput
```

协议、控制权与屏幕坐标的设计见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，完整工程计划见 [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md)。平台目录说明见 [platforms](platforms/README.md)。

## 下载预览版

如果只想使用已构建的程序，请从 [GitHub Releases](https://github.com/Henry-L1/InputMesh/releases) 下载：

- macOS Apple Silicon（arm64）：DMG 或 APP ZIP。
- Windows x64：NSIS 安装程序 EXE 或 MSI。
- 每个平台均附带 SHA-256 校验清单。

请只从本仓库的 Release 页面下载并核对 SHA-256。未签名安装包可能显示未知开发者或安全警告。

## 平台代码与发布

- 跨平台协议、网络与控制权逻辑位于 `src-tauri/src`。
- Windows 输入后端位于 `src-tauri/vendor/rdev/src/windows`，Windows 工具位于 `tools/windows`。
- macOS 输入后端位于 `src-tauri/vendor/rdev/src/macos`，macOS 工具位于 `tools/macos`。
- Windows 和 macOS 发布包必须使用相同的版本号，并分别在对应平台构建。

更多平台说明见 [platforms/windows](platforms/windows/README.md) 和 [platforms/macos](platforms/macos/README.md)。

## 当前限制

- 一个操作系统本质上只有一个系统指针；“多套键鼠”采用最近一次真实本地输入接管，而不是生成多个独立光标。
- 不转发剪贴板、文件、音频或视频。
- Windows 的 UAC 安全桌面和 macOS 登录窗口不会由普通用户进程控制。
- 每个发布候选版本仍需在真实 Windows 与 Mac 上复测；单机单元测试不能替代系统权限、休眠唤醒、防火墙和 Wi-Fi 延迟测试。

## 参考项目

- [Deskflow](https://github.com/deskflow/deskflow)：功能成熟，适合允许 GPL-2.0 的产品直接采用。
- [Input Leap](https://github.com/input-leap/input-leap)：已归档，可作历史行为参考。
- [Barrier](https://github.com/debauchee/barrier)：已停止维护，不适合做新项目基础。
- [Synergy](https://github.com/symless/synergy)：仍活跃，源码同样受 GPL-2.0 约束。

InputMesh 选择绿地实现，以避免引入上述项目的 GPL 源码；当前协议不承诺兼容上述项目。

## 许可

InputMesh 自有代码采用 [PolyForm Noncommercial License 1.0.0](LICENSE)：

- 允许个人学习、研究、测试、娱乐和业余项目等非商业用途。
- 允许为上述非商业目的查看、修改和分发源码。
- 不允许用于营利产品或服务、收费分发、商业内部使用、客户项目或其他商业目的。
- 第三方组件继续适用各自的许可证，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

本项目属于“源码可用（source-available）”，而不是 OSI 定义的开源软件。许可边界以 [LICENSE](LICENSE) 正文为准。

安全问题请阅读 [SECURITY.md](SECURITY.md)；贡献代码前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。
