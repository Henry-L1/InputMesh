# InputMesh v0.1.4

这是 Windows 与 macOS 同步发布的公开预览版本。两个平台的文件都来自同一份 v0.1.4 源码候选，并由仓库中的发布清单锁定文件名与 SHA-256。

## 主要修复

- 修复 Windows 鼠标进入 macOS 后，在远端屏幕边缘被吸住且无法返回的问题。
- 修复 Windows 低级 Hook 与 InputMesh `SendInput` 回注事件之间的重入死锁。
- 修复已吞掉的 Windows 鼠标事件错误累积基准，确保反向移动和多次跨屏持续有效。
- 为输入 TCP 连接启用 `TCP_NODELAY`，消除小型鼠标移动包被合并后造成的远端卡顿。
- 强化 Windows 虚拟桌面、多显示器、负坐标和物理 LAN 优先连接。

## 下载对应关系

| 系统 | 架构 | 推荐文件 | 备用格式 |
| --- | --- | --- | --- |
| Windows 10/11 | x64 | `InputMesh-v0.1.4-windows-x64-setup.exe` | `InputMesh-v0.1.4-windows-x64.msi` |
| macOS 12+ | Apple Silicon / arm64 | `InputMesh-v0.1.4-macos-arm64.dmg` | `InputMesh-v0.1.4-macos-arm64.app.zip` |

不要混用其他版本的客户端。所有文件都应显示版本 `0.1.4`，完整 SHA-256 位于仓库的 `releases/v0.1.4` 目录。

## 验证

- Windows Rust 测试：50/50。
- macOS Rust 测试：51/51。
- 两端 Clippy、前端测试和生产构建通过。
- 真实 Windows ↔ macOS 局域网连接、双向跨屏、键盘、回程、边缘释放、重连通过。
- 用户实机确认 Windows → macOS 鼠标移动流畅。

## 签名说明

这些文件仅供非商业测试。macOS 应用采用本地 ad-hoc 签名且未公证；Windows 安装包尚未进行 Authenticode 签名，因此系统可能显示安全提示。安装前请核对仓库中公布的 SHA-256。Windows UAC 安全桌面、休眠唤醒、Wi-Fi 断线恢复和长时间压力测试尚未完成公开发布级验证。
