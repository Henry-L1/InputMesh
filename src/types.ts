export type OsKind = "macos" | "windows" | "linux" | "unknown";
export type ServiceStatus = "stopped" | "starting" | "running" | "error";
export type PeerStatus = "discovered" | "pairing" | "connected" | "offline" | "rejected";

export interface ScreenInfo {
  id: string;
  nativeId: string;
  ownerDeviceId: string;
  ownerName: string;
  name: string;
  width: number;
  height: number;
  scaleFactor: number;
  x: number;
  y: number;
  primary: boolean;
  enabled: boolean;
  online: boolean;
}

export interface DeviceInfo {
  id: string;
  name: string;
  os: OsKind;
  version: string;
}

export interface PeerInfo extends DeviceInfo {
  status: PeerStatus;
  address?: string;
  latencyMs?: number;
  fingerprint?: string;
  pairingCode?: string;
  lastSeenAt: number;
  screens: ScreenInfo[];
}

export interface PermissionState {
  accessibility: boolean;
  inputMonitoring: boolean;
  requiresAction: boolean;
  helpText: string;
}

export interface SharingSettings {
  switchDelayMs: number;
  edgeResistancePx: number;
  takeControlOnLocalInput: boolean;
  launchAtLogin: boolean;
}

export interface ActivityLog {
  id: string;
  timestamp: number;
  level: "info" | "warning" | "error";
  message: string;
}

export interface AppSnapshot {
  localDevice: DeviceInfo;
  serviceStatus: ServiceStatus;
  sharingEnabled: boolean;
  activeControllerId: string;
  activeScreenId?: string;
  listenPort?: number;
  permission: PermissionState;
  screens: ScreenInfo[];
  peers: PeerInfo[];
  settings: SharingSettings;
  logs: ActivityLog[];
}
