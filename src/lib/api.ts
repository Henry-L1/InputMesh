import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActivityLog,
  AppSnapshot,
  PeerInfo,
  ScreenInfo,
  SharingSettings,
} from "../types";

type SnapshotListener = (snapshot: AppSnapshot) => void;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const demoListeners = new Set<SnapshotListener>();

const now = Date.now();

const localScreens: ScreenInfo[] = [
  {
    id: "screen-mac-main",
    nativeId: "CG-1",
    ownerDeviceId: "local-mac",
    ownerName: "Henry 的 Mac Studio",
    name: "Studio Display",
    width: 2560,
    height: 1440,
    scaleFactor: 2,
    x: 0,
    y: 0,
    primary: true,
    enabled: true,
    online: true,
  },
];

const windowsScreens: ScreenInfo[] = [
  {
    id: "screen-win-main",
    nativeId: "DISPLAY1",
    ownerDeviceId: "peer-windows",
    ownerName: "WIN-DEV",
    name: "Dell U2723QE",
    width: 3840,
    height: 2160,
    scaleFactor: 1.5,
    x: 2560,
    y: -360,
    primary: true,
    enabled: true,
    online: true,
  },
  {
    id: "screen-win-side",
    nativeId: "DISPLAY2",
    ownerDeviceId: "peer-windows",
    ownerName: "WIN-DEV",
    name: "竖屏显示器",
    width: 1440,
    height: 2560,
    scaleFactor: 1.25,
    x: 6400,
    y: -200,
    primary: false,
    enabled: true,
    online: true,
  },
];

const airScreens: ScreenInfo[] = [
  {
    id: "screen-air",
    nativeId: "CG-2",
    ownerDeviceId: "peer-air",
    ownerName: "办公 MacBook Air",
    name: "内建显示器",
    width: 2560,
    height: 1664,
    scaleFactor: 2,
    x: -2560,
    y: 100,
    primary: true,
    enabled: false,
    online: true,
  },
];

const offlineScreens: ScreenInfo[] = [
  {
    id: "screen-lab",
    nativeId: "DISPLAY1",
    ownerDeviceId: "peer-lab",
    ownerName: "LAB-PC",
    name: "实验室显示器",
    width: 1920,
    height: 1080,
    scaleFactor: 1,
    x: 7840,
    y: 0,
    primary: true,
    enabled: false,
    online: false,
  },
];

let demoSnapshot: AppSnapshot = {
  localDevice: {
    id: "local-mac",
    name: "Henry 的 Mac Studio",
    os: "macos",
    version: "macOS 15.6",
  },
  serviceStatus: "running",
  sharingEnabled: false,
  activeControllerId: "local-mac",
  activeScreenId: "screen-mac-main",
  listenPort: 42424,
  permission: {
    accessibility: true,
    inputMonitoring: false,
    requiresAction: true,
    helpText: "允许“输入监控”后，InputMesh 才能把键盘和鼠标事件发送到其他设备。",
  },
  screens: [...localScreens, ...windowsScreens, ...airScreens, ...offlineScreens],
  peers: [
    {
      id: "peer-windows",
      name: "WIN-DEV",
      os: "windows",
      version: "Windows 11 Pro",
      status: "connected",
      address: "192.168.50.32",
      latencyMs: 2,
      fingerprint: "72:5A:10:98:EC:31",
      lastSeenAt: now,
      screens: windowsScreens,
    },
    {
      id: "peer-air",
      name: "办公 MacBook Air",
      os: "macos",
      version: "macOS 15.5",
      status: "pairing",
      address: "192.168.50.48",
      latencyMs: 5,
      pairingCode: "391527",
      lastSeenAt: now - 12_000,
      screens: airScreens,
    },
    {
      id: "peer-lab",
      name: "LAB-PC",
      os: "windows",
      version: "Windows 10",
      status: "offline",
      address: "192.168.50.77",
      lastSeenAt: now - 1000 * 60 * 37,
      screens: offlineScreens,
    },
  ],
  settings: {
    switchDelayMs: 180,
    edgeResistancePx: 12,
    takeControlOnLocalInput: true,
    launchAtLogin: false,
  },
  logs: [
    {
      id: "log-1",
      timestamp: now - 11_000,
      level: "info",
      message: "已发现设备“办公 MacBook Air”",
    },
    {
      id: "log-2",
      timestamp: now - 1000 * 60 * 3,
      level: "info",
      message: "控制已切换到 Henry 的 Mac Studio · Studio Display",
    },
    {
      id: "log-3",
      timestamp: now - 1000 * 60 * 6,
      level: "warning",
      message: "输入监控权限尚未开启",
    },
    {
      id: "log-4",
      timestamp: now - 1000 * 60 * 14,
      level: "info",
      message: "与 WIN-DEV 建立安全连接，往返延迟 2 ms",
    },
  ],
};

export const isTauriRuntime =
  typeof window !== "undefined" && window.__TAURI_INTERNALS__ != null;

function cloneSnapshot(snapshot: AppSnapshot) {
  return JSON.parse(JSON.stringify(snapshot)) as AppSnapshot;
}

function isSnapshot(value: unknown): value is AppSnapshot {
  return Boolean(
    value &&
      typeof value === "object" &&
      "localDevice" in value &&
      "serviceStatus" in value &&
      "screens" in value,
  );
}

function addLog(
  snapshot: AppSnapshot,
  message: string,
  level: ActivityLog["level"] = "info",
) {
  const entry: ActivityLog = {
    id: `log-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    timestamp: Date.now(),
    level,
    message,
  };
  return { ...snapshot, logs: [entry, ...snapshot.logs].slice(0, 200) };
}

function commitDemo(
  update: (snapshot: AppSnapshot) => AppSnapshot,
  message?: string,
  level?: ActivityLog["level"],
) {
  demoSnapshot = update(cloneSnapshot(demoSnapshot));
  if (message) demoSnapshot = addLog(demoSnapshot, message, level);
  const cloned = cloneSnapshot(demoSnapshot);
  demoListeners.forEach((listener) => listener(cloned));
  return cloned;
}

async function invokeAndRefresh(command: string, args?: Record<string, unknown>) {
  const result = await invoke<unknown>(command, args);
  if (isSnapshot(result)) return result;
  return invoke<AppSnapshot>("get_snapshot");
}

function updatePeerScreens(
  peers: PeerInfo[],
  screenId: string,
  update: (screen: ScreenInfo) => ScreenInfo,
) {
  return peers.map((peer) => ({
    ...peer,
    screens: peer.screens.map((screen) => (screen.id === screenId ? update(screen) : screen)),
  }));
}

export async function getSnapshot(): Promise<AppSnapshot> {
  if (!isTauriRuntime) return cloneSnapshot(demoSnapshot);
  return invoke<AppSnapshot>("get_snapshot");
}

export async function setSharingEnabled(enabled: boolean): Promise<AppSnapshot> {
  if (isTauriRuntime) {
    return invokeAndRefresh("set_sharing_enabled", { enabled });
  }
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      sharingEnabled: enabled,
    }),
    enabled ? "键鼠共享服务已开启" : "键鼠共享服务已暂停",
  );
}

export async function setScreenEnabled(screenId: string, enabled: boolean) {
  if (isTauriRuntime) {
    return invokeAndRefresh("set_screen_enabled", { screenId, enabled });
  }
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      screens: snapshot.screens.map((screen) =>
        screen.id === screenId ? { ...screen, enabled } : screen,
      ),
      peers: updatePeerScreens(snapshot.peers, screenId, (screen) => ({ ...screen, enabled })),
    }),
    `${demoSnapshot.screens.find((screen) => screen.id === screenId)?.name ?? "屏幕"} 输入共享已${enabled ? "启用" : "停用"}`,
  );
}

export async function updateScreenPosition(screenId: string, x: number, y: number) {
  if (isTauriRuntime) {
    return invokeAndRefresh("update_screen_position", { screenId, x, y });
  }
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      screens: snapshot.screens.map((screen) =>
        screen.id === screenId ? { ...screen, x, y } : screen,
      ),
      peers: updatePeerScreens(snapshot.peers, screenId, (screen) => ({ ...screen, x, y })),
    }),
    "屏幕布局已更新",
  );
}

export async function updateSettings(settings: SharingSettings) {
  if (isTauriRuntime) {
    return invokeAndRefresh("update_settings", { settings });
  }
  return commitDemo(
    (snapshot) => ({ ...snapshot, settings }),
    "偏好设置已保存",
  );
}

export async function pairPeer(peerId: string) {
  if (isTauriRuntime) {
    return invokeAndRefresh("pair_peer", { peerId });
  }
  const peer = demoSnapshot.peers.find((candidate) => candidate.id === peerId);
  const nextStatus = peer?.status === "pairing" ? "connected" : "pairing";
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      peers: snapshot.peers.map((candidate) =>
        candidate.id === peerId
          ? {
              ...candidate,
              status: nextStatus,
              pairingCode: nextStatus === "pairing" ? "391527" : undefined,
            }
          : candidate,
      ),
    }),
    nextStatus === "pairing"
      ? `已向 ${peer?.name ?? "设备"} 发起配对`
      : `${peer?.name ?? "设备"} 配对成功`,
  );
}

export async function rejectPeer(peerId: string) {
  if (isTauriRuntime) {
    return invokeAndRefresh("reject_peer", { peerId });
  }
  const peer = demoSnapshot.peers.find((candidate) => candidate.id === peerId);
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      peers: snapshot.peers.map((candidate) =>
        candidate.id === peerId ? { ...candidate, status: "rejected" } : candidate,
      ),
    }),
    `已断开或忽略 ${peer?.name ?? "设备"}`,
  );
}

export async function openPermissionSettings() {
  if (isTauriRuntime) {
    await invoke("open_permission_settings");
    return getSnapshot();
  }
  return commitDemo(
    (snapshot) => ({
      ...snapshot,
      permission: {
        ...snapshot.permission,
        accessibility: true,
        inputMonitoring: true,
        requiresAction: false,
        helpText: "InputMesh 已拥有键鼠共享所需权限。",
      },
    }),
    "输入监控权限已授权",
  );
}

export async function refreshDiscovery() {
  if (isTauriRuntime) {
    return invokeAndRefresh("refresh_discovery");
  }
  return commitDemo(
    (snapshot) => snapshot,
    "已完成一轮局域网设备扫描",
  );
}

export async function subscribeToSnapshot(listener: SnapshotListener): Promise<UnlistenFn> {
  if (isTauriRuntime) {
    return listen<AppSnapshot>("snapshot://updated", (event) => listener(event.payload));
  }
  demoListeners.add(listener);
  return () => demoListeners.delete(listener);
}
