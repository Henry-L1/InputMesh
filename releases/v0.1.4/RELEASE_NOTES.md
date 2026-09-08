# InputMesh v0.1.4

这是 Windows 与 macOS 同步验证过的源码预览版本。公开前依赖审计发现旧二进制所用锁文件包含已修复的传递依赖漏洞，因此预构建安装包已经撤下。请从当前源码构建；新的签名安装包将在重新构建和验证后发布。

## 主要修复

- 修复 Windows 鼠标进入 macOS 后，在远端屏幕边缘被吸住且无法返回的问题。
- 修复 Windows 低级 Hook 与 InputMesh `SendInput` 回注事件之间的重入死锁。
- 修复已吞掉的 Windows 鼠标事件错误累积基准，确保反向移动和多次跨屏持续有效。
- 为输入 TCP 连接启用 `TCP_NODELAY`，消除小型鼠标移动包被合并后造成的远端卡顿。
- 强化 Windows 虚拟桌面、多显示器、负坐标和物理 LAN 优先连接。

## 下载说明

当前 Release 仅提供 GitHub 自动生成的源码压缩包，不提供预构建安装包。请按照仓库 README 在对应操作系统上从源码构建。`releases/v0.1.4` 中的旧 SHA-256 仅保留为测试记录，不代表当前可下载文件。

## 验证

- Windows Rust 测试：50/50。
- macOS Rust 测试：51/51。
- 两端 Clippy、前端测试和生产构建通过。
- 真实 Windows ↔ macOS 局域网连接、双向跨屏、键盘、回程、边缘释放、重连通过。
- 用户实机确认 Windows → macOS 鼠标移动流畅。

## 签名说明

此前的 macOS 应用采用本地 ad-hoc 签名且未公证，Windows 安装包也未进行 Authenticode 签名，因此已经从公开 Release 撤下。Windows UAC 安全桌面、休眠唤醒、Wi-Fi 断线恢复和长时间压力测试尚未完成公开发布级验证。
