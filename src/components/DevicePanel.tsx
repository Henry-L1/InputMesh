import {
  Check,
  Clock3,
  Copy,
  Laptop,
  Monitor,
  RefreshCw,
  ShieldCheck,
  Unplug,
  Wifi,
  WifiOff,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { OsKind, PeerInfo } from "../types";

interface DevicePanelProps {
  peers: PeerInfo[];
  pairingCode?: string;
  onPair: (peerId: string) => Promise<void>;
  onReject: (peerId: string) => Promise<void>;
  onRefresh: () => Promise<void>;
}

const statusLabels: Record<PeerInfo["status"], string> = {
  discovered: "正在建立连接",
  pairing: "核对配对码",
  connected: "已连接",
  offline: "离线",
  rejected: "已忽略",
};

function OsGlyph({ os }: { os: OsKind }) {
  return os === "macos" ? <Laptop size={19} /> : <Monitor size={19} />;
}

export function DevicePanel({
  peers,
  pairingCode,
  onPair,
  onReject,
  onRefresh,
}: DevicePanelProps) {
  const [pendingPeer, setPendingPeer] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [copied, setCopied] = useState(false);
  const sortedPeers = useMemo(
    () =>
      [...peers].sort((left, right) => {
        const order: Record<PeerInfo["status"], number> = {
          pairing: 0,
          discovered: 1,
          connected: 2,
          offline: 3,
          rejected: 4,
        };
        return order[left.status] - order[right.status];
      }),
    [peers],
  );

  const pair = async (peerId: string) => {
    setPendingPeer(peerId);
    try {
      await onPair(peerId);
    } finally {
      setPendingPeer(null);
    }
  };

  const reject = async (peerId: string) => {
    setPendingPeer(peerId);
    try {
      await onReject(peerId);
    } finally {
      setPendingPeer(null);
    }
  };

  const refresh = async () => {
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  };

  const copyPairingCode = async () => {
    if (!pairingCode) return;
    try {
      await navigator.clipboard?.writeText(pairingCode.replace(/\s/g, ""));
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="devices-layout">
      <section className="pairing-card panel-card">
        <div className="pairing-card__glow" />
        <div className="eyebrow">当前配对码</div>
        <h2>{pairingCode ? pairingCode.replace(/(\d{3})(\d{3})/, "$1 $2") : "— — —"}</h2>
        <p>在另一台电脑上选择本机，并确认显示的六位数字一致。</p>
        <button
          className="button button--quiet"
          type="button"
          onClick={copyPairingCode}
          disabled={!pairingCode}
        >
          {copied ? <Check size={15} /> : <Copy size={15} />}
          {copied ? "已复制" : "复制配对码"}
        </button>
        <div className="pairing-card__security">
          <ShieldCheck size={15} />
          配对请求和输入流均经过端到端加密
        </div>
      </section>

      <section className="device-list panel-card">
        <div className="panel-card__heading">
          <div>
            <span className="eyebrow">自动发现</span>
            <h2>局域网设备</h2>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label="刷新局域网设备"
            title="刷新设备"
            onClick={refresh}
            disabled={refreshing}
          >
            <RefreshCw size={17} className={refreshing ? "spin" : ""} />
          </button>
        </div>

        <div className="device-list__items">
          {sortedPeers.length === 0 ? (
            <div className="empty-state">
              <Wifi size={24} />
              <strong>未发现其他设备</strong>
              <span>确认两台电脑处于同一个局域网。</span>
            </div>
          ) : (
            sortedPeers.map((peer) => {
              const pending = pendingPeer === peer.id;
              return (
                <article className={`peer-row peer-row--${peer.status}`} key={peer.id}>
                  <div className="peer-row__icon">
                    <OsGlyph os={peer.os} />
                    <span className={`presence presence--${peer.status}`} />
                  </div>
                  <div className="peer-row__body">
                    <div className="peer-row__title">
                      <strong>{peer.name}</strong>
                      <span className={`status-tag status-tag--${peer.status}`}>
                        {statusLabels[peer.status]}
                      </span>
                    </div>
                    <div className="peer-row__meta">
                      <span>{peer.os === "macos" ? "macOS" : peer.os === "windows" ? "Windows" : peer.os}</span>
                      {peer.address && <span>{peer.address}</span>}
                      {peer.latencyMs != null && (
                        <span className="latency">
                          <Wifi size={11} /> {peer.latencyMs} ms
                        </span>
                      )}
                      {peer.status === "offline" && (
                        <span>
                          <Clock3 size={11} /> 最近在线 {formatRelativeTime(peer.lastSeenAt)}
                        </span>
                      )}
                    </div>

                    {peer.status === "pairing" && peer.pairingCode && (
                      <div className="peer-row__code">
                        <span>确认两端均显示</span>
                        <strong>{peer.pairingCode.replace(/(\d{3})(\d{3})/, "$1 $2")}</strong>
                      </div>
                    )}
                  </div>
                  <div className="peer-row__actions">
                    {peer.status === "discovered" && (
                      <RefreshCw className="spin muted-icon" size={17} aria-label="正在连接" />
                    )}
                    {peer.status === "pairing" && (
                      <button
                        type="button"
                        className="button button--primary button--small"
                        disabled={pending}
                        onClick={() => pair(peer.id)}
                      >
                        <Check size={14} /> 确认一致
                      </button>
                    )}
                    {peer.status === "connected" && (
                      <button
                        type="button"
                        className="button button--quiet button--small"
                        disabled={pending}
                        onClick={() => reject(peer.id)}
                      >
                        <Unplug size={14} /> 取消信任
                      </button>
                    )}
                    {peer.status === "offline" && <WifiOff className="muted-icon" size={17} />}
                  </div>
                </article>
              );
            })
          )}
        </div>
      </section>
    </div>
  );
}

function formatRelativeTime(timestamp: number) {
  const deltaMinutes = Math.max(1, Math.round((Date.now() - timestamp) / 60_000));
  if (deltaMinutes < 60) return `${deltaMinutes} 分钟前`;
  const hours = Math.round(deltaMinutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.round(hours / 24)} 天前`;
}
