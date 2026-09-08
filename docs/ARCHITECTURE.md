# InputMesh 架构

## 设计原则

InputMesh 把显示器而不是电脑作为用户交互单位。每块屏幕有全局 ID、所属设备、原生像素尺寸和用户定义的画布坐标。只有 `enabled && online` 的屏幕会参与命中检测和跨边切换。

控制面（React/Tauri commands）只负责展示和修改状态；数据面（Rust）负责发现、认证、事件路由和系统输入。系统输入回调里不做磁盘或网络阻塞操作，事件通过内存队列进入会话任务。

## 组件

| 组件 | 职责 |
|---|---|
| `model` | UI 与 Rust 共用的数据契约 |
| `config` | 身份密钥、信任关系、屏幕布局和设置的原子持久化 |
| `topology` | 全局坐标、屏幕命中、不同分辨率间的边缘映射 |
| `discovery` | `_inputmesh._tcp.local.` mDNS 注册和浏览 |
| `crypto` | Noise XX 握手、配对码、加密帧和尺寸限制 |
| `protocol` | 可版本化的 hello、屏幕、配对、控制权、焦点、输入与心跳消息 |
| `network` | TCP listener、连接生命周期、可信公钥校验、收发队列 |
| `input` | macOS/Windows 全局捕获、抑制、注入与回环过滤 |
| `runtime` | 汇总状态、Tauri 事件、控制权租约和故障降级 |

## 首次配对与重连

```text
A discovers B over mDNS
        │
        ├── TCP + Noise XX (ephemeral + static keys)
        │
        ├── both derive the same 6-digit short authentication string
        │
        ├── users compare and approve on both computers
        │
        └── each side stores peer ID → Noise static public key

Reconnect: complete Noise XX → verify stored static public key → enable input messages
```

配对码只用于人工核对，不作为低熵加密密钥。Noise 会话提供机密性、完整性和前向保密；发现广播中不发送密钥或输入内容。协议帧有硬性大小上限，避免长度字段导致无界分配。

## 控制权与多套输入

任何节点检测到非注入的物理输入时，都可以发布一个新的控制权 claim。claim 是 `(generation, ownerDeviceId)`；节点先把自己见过的最大 generation 加一，同 generation 冲突时用设备 ID 做确定性比较。输入事件必须携带当前 claim，接收者丢弃旧 claim 的事件。

这实现了“用户在哪一端操作就由哪一端接管”，但不是多人同时操作模式。同一时刻只有一个逻辑控制者和一个系统焦点，从而避免两只鼠标争抢一个 OS 指针。

## 坐标与跨屏

平台层捕获原生绝对位置，运行时换算为当前屏幕内的归一化坐标。移动越过屏幕边缘时，拓扑模块选择该方向上距离最近且有重叠边的屏幕；沿边的位置按比例映射到不同分辨率的目标边。

远端指针消息同时携带目标 `screenId` 与归一化位置。接收端用本机刚枚举的原生屏幕矩形换算最终注入坐标，不信任发送端提供的绝对桌面坐标。

## 输入回环

注入事件也可能被全局钩子再次看到。平台层在注入前把事件放入短期队列，捕获回调只消费匹配且未过期的事件；这些事件不会触发接管或再次发送。Windows 额外使用 `dwExtraInfo` 标记 `SendInput` 回注事件，macOS 使用事件 source/user-data 标记进一步强化识别。

## 故障安全

- 会话断开或心跳超时立即把焦点拉回控制者本机的主屏。
- 未连接、未启用、离线或公钥不匹配的屏幕不参与拓扑。
- `Ctrl + Alt + Esc` 在捕获回调中本地处理，停止抑制而不依赖网络/UI。
- 未授予系统权限时可以管理拓扑和配对，但不能打开共享。
- 配置写入临时文件、同步后原子替换；Unix 上身份配置权限设置为 `0600`。
