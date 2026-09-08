# InputMesh 0.1.4 Windows 联调报告

日期：2026-09-07 至 2026-09-08

## 环境

- macOS 主机：`198.51.100.38`（文档示例地址）
- Windows 主机：`Windows-Test-PC`，Windows 10.0.26200，交互会话 6，局域网地址 `198.51.100.37`（文档示例地址）
- Windows 工具链：Rust 1.98.1、Node.js 26.7.0、pnpm 11.19.0、Visual Studio Build Tools 2022 17.14.39、WebView2 152.0.4191.66
- 传输：Noise XX 加密 TCP；mDNS `_inputmesh._tcp` 自动发现

## 结果

| 检查项 | 结果 | 证据 |
| --- | --- | --- |
| Rust 测试 | 通过 | Windows 50/50；macOS 51/51 |
| Rust 静态检查 | 通过 | 两端 `cargo clippy --all-targets -- -D warnings` |
| 前端测试与构建 | 通过 | Windows/macOS 各 3/3，TypeScript 与 Vite 生产构建通过 |
| Windows 安装包 | 通过 | exe、MSI、NSIS 均由 Tauri release 构建生成 |
| 自动发现与配对 | 通过 | 两端均发现对方，首次确认后重启仍保持信任与共享状态 |
| 物理 LAN 直连 | 通过 | 两台测试主机之间的 TCP 连接为 `ESTABLISHED`（地址已匿名化） |
| 屏幕拓扑 | 通过 | Mac 1 块屏幕与 Windows 2 块屏幕同步为同一画布 |
| Windows 多屏注入 | 通过 | 虚拟桌面 `(0,-209) 3640x1920`；目标 `(1710,1136)`，观察到 `(1709,1136)` |
| 回环抑制标记 | 通过 | Windows 低层 Hook 识别到 InputMesh `dwExtraInfo` 标记 |
| Mac → Windows 指针 | 通过 | Windows 日志记录远程焦点进入两块 Windows 屏幕 |
| Windows → Mac 指针 | 通过 | Windows 日志记录从本机屏幕跨至 Mac 屏幕 |
| Mac → Windows 键盘 | 通过 | Windows 收包日志记录 F9 `pressed=true/false`，交互桌面 `GetAsyncKeyState` 观察到 `down/up` |
| 重启恢复 | 通过 | Windows 重启后自动恢复共享并重新连接；Mac 替换构建后配对仍保留 |
| 远端指针流畅度 | 通过 | 两端启用 `TCP_NODELAY`；Windows 日志确认生效，用户在 Windows → Mac 实机操作中确认流畅 |

最终 Windows 输入探针输出：

```json
{"ok":true,"syntheticMarker":true,"virtualDesktop":{"x":0,"y":-209,"width":3640,"height":1920},"target":{"x":1710,"y":1136},"observed":{"x":1709,"y":1136}}
```

最终跨机键盘探针输出：

```json
{"ok":true,"key":"F9","states":["down","up"],"source":"GetAsyncKeyState"}
```

## 2026-09-08 Windows 回程卡死与 Mac 边缘吸附回归

- 根因一：从远端屏幕返回 Windows 时，低级 Hook 回调内部同步调用 `SendInput`；回注事件再次进入仍被占用的回调锁，造成 Hook 线程永久自锁。
- 根因二：远端控制期间，已被 Hook 吞掉的 Windows 鼠标事件仍被当作下一次相对位移的基准；真实本地光标并未移动，连续运动会退化为零位移，撞到 Mac 外边缘后反向移动表现为被吸住。
- 修复：带 InputMesh 标记的回注事件在 Hook 层更新鼠标基准后直接放行，不再进入应用回调；物理鼠标事件被吞掉时，用未改变的真实 Windows 光标位置重置下一次相对位移基准。
- Hook 内重入探针通过：`reentrantSendInput=true`，目标 `(1903,527)`，观察到 `(1903,526)`。
- 连续吞事件探针通过：第一次相对位移 `(33,1)`，第二次仍为 `(33,1)`；旧实现第二次为零。
- 真实 LAN 往返日志记录 Windows → Mac → Windows → Mac → Windows，多次回程后 Hook 仍继续处理输入。
- Windows Rust 测试 `49/49`，macOS Rust 测试 `50/50`，两端 Clippy、前端 `3/3` 与生产构建通过。

## 2026-09-08 远端鼠标移动流畅度修复

- 根因：每个细小指针移动都作为独立的加密 TCP 帧发送，但连接未禁用 Nagle；小包可能等待确认或被合并后成批抵达，表现为远端鼠标移动不连续。
- 修复：每条 InputMesh TCP 连接在安全握手前启用 `TCP_NODELAY`，失败时停止使用该连接并报告明确错误；键盘、点击与指针事件的原有顺序语义不变。
- 自动验证：新增双端 socket 测试，确认连接两端均能启用并读回 `TCP_NODELAY=true`；Windows `50/50`、macOS `51/51` Rust 测试通过，两端严格 Clippy 通过，前端 `3/3` 与生产构建通过。
- 实机验证：Windows 运行日志记录 `enabled TCP_NODELAY for input transport`；两台测试主机之间的局域网连接保持 `ESTABLISHED`；用户在 Windows → Mac 连续移动中确认“可以了很流畅”。

## 本轮 Windows 修复

- 所有 `SendInput` 鼠标和键盘事件携带 InputMesh 专用标记，低层 Hook 忽略回注事件，避免控制反馈环。
- 绝对鼠标坐标按 Windows 虚拟桌面原点与尺寸归一化，支持负坐标和多显示器。
- Windows 鼠标 Hook 计算连续事件的相对位移，跨屏时不再依赖绝对坐标差猜测方向。
- Windows Hook 使用线程安全回调槽；消息泵使用有效 `MSG` 并持续分派消息。
- 同时存在物理 LAN 和 VPN 地址时，连接地址优先匹配操作系统路由选出的本地接口与最长共同前缀。

## 产物

- 远程开发目录：`C:\Users\tester\InputMesh`
- Windows 当前运行程序：`C:\Users\tester\AppData\Local\InputMesh\inputmesh.exe`
- Windows bundle 阶段主程序：`C:\Users\tester\InputMesh\src-tauri\target\release\inputmesh.exe`
- MSI 安装包：`C:\Users\tester\InputMesh\src-tauri\target\release\bundle\msi\InputMesh_0.1.4_x64_en-US.msi`
- NSIS 安装包：`C:\Users\tester\Desktop\InputMesh_0.1.4_x64-setup.exe`
- 当前运行 exe SHA-256：`497BE76B1E064C2ACFC23BE8523E247F104E08A95B427BA5B501D81D6C79AC31`
- bundle 阶段 exe SHA-256：`E03A6C314DB31BE4BCC9D2028EC0346B85204F445D59330F19B578BC548298CC`
- MSI SHA-256：`581536BA31D9B3EF4515F24C904C7A6267665E73D696E2D77EE19DBB6283ECA2`
- NSIS SHA-256：`C832D3CC452FFDF9FCEC28F57DDC892A27702154F4684668BF1452D795175E7F`
- macOS 当前运行程序：`/Applications/InputMesh.app`，主二进制 SHA-256：`8bd0ca6258112b7c479d1423dfd16d05702b157a636df6e7b8ff5210dfd7a563`
- macOS DMG：`src-tauri/target/release/bundle/dmg/InputMesh_0.1.4_aarch64.dmg`，SHA-256：`77985272da530e515243afc28da852db542e2d4ae299d301cde1a640193f0294`

GitHub Release 使用统一标签 `v0.1.4`，并按 `releases/v0.1.4/windows` 与 `releases/v0.1.4/macos` 分开记录两个平台的发布文件名和校验值。

该安装包是未签名开发构建，仅用于当前受信任局域网内测试。公开分发前仍需 Windows 代码签名，并补做 UAC 安全桌面、休眠唤醒、Wi-Fi 断连恢复和 10 分钟持续输入压力测试。
