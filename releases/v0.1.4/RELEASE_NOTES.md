# InputMesh v0.1.4

这是 Windows 与 macOS 同步验证过的非商业预览版本。旧二进制已在依赖审计后撤下；当前安装包由 GitHub Actions 从已修复的 `v0.1.4` 源码重新构建并附带 SHA-256 校验清单。

## 主要修复

- 修复 Windows 鼠标进入 macOS 后，在远端屏幕边缘被吸住且无法返回的问题。
- 修复 Windows 低级 Hook 与 InputMesh `SendInput` 回注事件之间的重入死锁。
- 修复已吞掉的 Windows 鼠标事件错误累积基准，确保反向移动和多次跨屏持续有效。
- 为输入 TCP 连接启用 `TCP_NODELAY`，消除小型鼠标移动包被合并后造成的远端卡顿。
- 强化 Windows 虚拟桌面、多显示器、负坐标和物理 LAN 优先连接。

## 下载说明

- macOS Apple Silicon：DMG 或 APP ZIP。
- Windows x64：NSIS 安装程序 EXE 或 MSI。
- 每个平台均提供 `.sha256` 校验文件；仓库内的 `releases/v0.1.4` 同步记录相同哈希。

## 验证

- Windows Rust 测试通过。
- macOS Rust 测试：51/51。
- 两端 Clippy、前端测试和生产构建通过。
- RustSec 依赖审计通过，当前无已知 Rust 依赖漏洞。
- 真实 Windows ↔ macOS 局域网连接、双向跨屏、键盘、回程、边缘释放、重连通过。
- 用户实机确认 Windows → macOS 鼠标移动流畅。

## 签名说明

当前 macOS 应用采用 ad-hoc 签名且未公证，Windows 安装包也未进行 Authenticode 签名，因此本版本保持 prerelease 状态，系统可能显示未知开发者警告。Windows UAC 安全桌面、休眠唤醒、Wi-Fi 断线恢复和长时间压力测试尚未完成公开发布级验证。
