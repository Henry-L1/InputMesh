# InputMesh

InputMesh 是一个面向 Windows 与 macOS 的点对点键鼠共享应用。把应用装到同一局域网内的电脑后，可以在一个画布上排列所有屏幕；鼠标越过屏幕边缘时，键盘焦点会一起切换到目标电脑。

项目源码公开，项目自有代码采用 PolyForm Noncommercial 1.0.0 许可，仅允许个人及其他非商业用途。项目没有复制 Deskflow、Input Leap、Barrier 或 Synergy 的 GPL 源码。

## 第一版目标

- Windows 10/11 与 macOS 12+ 使用同一套 Tauri 2 / Rust 工程。
- 用 mDNS 自动发现局域网内的 InputMesh 实例。
- 用 Noise XX 握手建立端到端加密连接，并以双方相同的 6 位数字进行首次配对确认。
- 自动枚举每台电脑的显示器，支持拖动排列和逐屏启用/停用。
- 鼠标跨屏后转发鼠标、滚轮和键盘事件；键盘按物理键码转发，默认遵循 Windows 风格布局，不同步输入法布局。
- 任一电脑上的本地物理输入都能按“最近操作者优先”接管控制。
- `Ctrl + Alt + Esc` 是本地紧急释放快捷键。

## 架构

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

协议、控制权与屏幕坐标的设计见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，完整工程计划见 [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md)。

## 本地开发

前置条件：

- Node.js 24 与 pnpm 11
- Rust stable
- macOS：Xcode Command Line Tools
- Windows：MSVC C++ Build Tools 与 WebView2 Runtime

```bash
pnpm install
pnpm tauri dev
```

只预览控制界面时可运行 `pnpm dev`；浏览器模式会使用内置演示数据，不会捕获真实键鼠。

验证工程：

```bash
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

构建当前平台安装包：

```bash
pnpm tauri build
```

Windows 和 macOS 安装包应分别在对应系统或 CI runner 上构建。未签名的本地包只适合开发测试；公开分发前还需要 Apple Developer ID 签名/公证及 Windows 代码签名。

## 平台目录与发布对应关系

- 跨平台协议、网络与控制权逻辑保留在 `src-tauri/src`，避免两端协议分叉。
- Windows 平台输入源码在 `src-tauri/vendor/rdev/src/windows`，Windows 专用工具在 `tools/windows`。
- macOS 平台输入源码在 `src-tauri/vendor/rdev/src/macos`，macOS 专用工具在 `tools/macos`。
- 每个 GitHub Release 必须同时发布同一版本号的 Windows 与 macOS 产物；`releases/vX.Y.Z` 中的清单固定文件名与 SHA-256，CI 会拒绝版本错位。

当前平台入口和发布文件说明见 [platforms](platforms/README.md) 与 [v0.1.4 发布清单](releases/v0.1.4/README.md)。

## 两台电脑联调

1. 两台电脑连接到同一个可信局域网并启动 InputMesh。
2. macOS 在“系统设置 → 隐私与安全性”中授予 InputMesh“辅助功能”和“输入监控”权限，然后重启应用。
3. 在双方界面核对 6 位配对码，确认一致后分别点击“允许”。
4. 在屏幕画布中拖动显示器，使相邻边与真实摆放一致；关掉不希望纳管的屏幕。
5. 打开“开始共享”，再把鼠标推过配置好的相邻边缘。

macOS 15 及以上还可能要求“本地网络”权限。Windows 防火墙首次提示时，仅应在受信任的专用网络上允许访问。默认 TCP 监听端口从 `42424` 开始自动选择。

## 可借鉴的开源项目

- [Deskflow](https://github.com/deskflow/deskflow)：当前活跃、功能成熟，适合允许 GPL-2.0 的产品直接采用。
- [Input Leap](https://github.com/input-leap/input-leap)：已归档，可作历史行为参考。
- [Barrier](https://github.com/debauchee/barrier)：已停止维护，不适合做新项目基础。
- [Synergy](https://github.com/symless/synergy)：仍活跃，源码同样受 GPL-2.0 约束。

InputMesh 选择绿地实现，以避免引入上述项目的 GPL 源码。这里的协议目前不承诺兼容上述项目。

## 安全说明

发现不等于信任。未由用户确认的设备不能接收或注入输入；已配对设备的 Noise 静态公钥保存在本机配置中。详细威胁模型与报告方式见 [SECURITY.md](SECURITY.md)。请不要在不受信任的公共 Wi-Fi 上启用键鼠共享。

## 当前限制

- 一套操作系统本质上只有一个系统指针；“多套键鼠”采用最近一次真实本地输入接管，而不是生成多个独立光标。
- 不转发剪贴板、文件、音频或视频。
- Windows 的 UAC 安全桌面和 macOS 登录窗口不会由普通用户进程控制。
- 每个发布候选版本仍需在真实 Windows 与 Mac 上复测；单机单元测试不能替代系统权限、休眠唤醒、防火墙和 Wi-Fi 延迟测试。

## 使用许可

InputMesh 自有代码采用 [PolyForm Noncommercial License 1.0.0](LICENSE)：

- 允许个人学习、研究、测试、娱乐和业余项目等非商业用途。
- 允许为上述非商业目的查看、修改和分发源码。
- 不允许用于营利产品或服务、收费分发、商业内部使用、客户项目或其他商业目的。
- 第三方组件继续适用各自的许可证，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

本项目因此属于“源码可用（source-available）”，而不是 OSI 定义的开源软件。许可边界以 [LICENSE](LICENSE) 正文为准。
