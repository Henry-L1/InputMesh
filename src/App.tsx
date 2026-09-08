import {
  Activity,
  AlertCircle,
  ArrowRight,
  Check,
  ChevronRight,
  CircleHelp,
  Copy,
  Cpu,
  ExternalLink,
  Keyboard,
  LayoutDashboard,
  ListFilter,
  LoaderCircle,
  Monitor,
  MousePointer2,
  Network,
  Radio,
  RefreshCw,
  Settings,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Wifi,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { DevicePanel } from "./components/DevicePanel";
import { ScreenTopology } from "./components/ScreenTopology";
import { SettingsPanel } from "./components/SettingsPanel";
import { Toggle } from "./components/Toggle";
import * as api from "./lib/api";
import type { ActivityLog, AppSnapshot, SharingSettings } from "./types";

type PageId = "layout" | "devices" | "settings" | "logs";
type LogFilter = "all" | ActivityLog["level"];

interface NavItem {
  id: PageId;
  label: string;
  icon: LucideIcon;
}

const navItems: NavItem[] = [
  { id: "layout", label: "控制台", icon: LayoutDashboard },
  { id: "devices", label: "设备", icon: Network },
  { id: "settings", label: "偏好设置", icon: Settings },
  { id: "logs", label: "活动日志", icon: Activity },
];

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [page, setPage] = useState<PageId>("layout");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    api
      .getSnapshot()
      .then((next) => {
        if (!cancelled) setSnapshot(next);
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(errorMessage(reason));
      });

    api
      .subscribeToSnapshot((next) => {
        if (!cancelled) setSnapshot(next);
      })
      .then((cleanup) => {
        if (cancelled) cleanup();
        else unlisten = cleanup;
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(errorMessage(reason));
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const refreshAfterPermissionChange = () => {
      if (!api.isTauriRuntime) return;
      api.getSnapshot().then(setSnapshot).catch((reason: unknown) => {
        setError(errorMessage(reason));
      });
    };
    window.addEventListener("focus", refreshAfterPermissionChange);
    return () => window.removeEventListener("focus", refreshAfterPermissionChange);
  }, []);

  const run = async (
    key: string,
    operation: () => Promise<AppSnapshot>,
  ) => {
    setBusy(key);
    setError(null);
    try {
      const next = await operation();
      setSnapshot(next);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(null);
    }
  };

  if (!snapshot) {
    return (
      <main className="boot-screen">
        <BrandMark />
        <LoaderCircle className="spin" size={22} />
        <p>{error ? "无法连接 InputMesh 服务" : "正在准备键鼠共享…"}</p>
        {error && <span>{error}</span>}
      </main>
    );
  }

  const connectedPeers = snapshot.peers.filter((peer) => peer.status === "connected");
  const discoveredPeers = snapshot.peers.filter(
    (peer) => peer.status === "discovered" || peer.status === "pairing",
  );
  const serviceIsOn = snapshot.sharingEnabled && snapshot.serviceStatus !== "stopped";
  const serviceLabel =
    snapshot.serviceStatus === "error"
      ? "服务异常"
      : snapshot.serviceStatus === "starting"
        ? "正在启动"
        : snapshot.sharingEnabled
          ? "共享运行中"
          : "共享已暂停";
  const activePairingCode = snapshot.peers.find(
    (peer) => peer.status === "pairing" && peer.pairingCode,
  )?.pairingCode;

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar__brand">
          <BrandMark />
          <div>
            <strong>InputMesh</strong>
            <span>跨设备键鼠</span>
          </div>
        </div>

        <nav className="sidebar__nav" aria-label="主导航">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                aria-current={page === item.id ? "page" : undefined}
                className={page === item.id ? "is-active" : ""}
                onClick={() => setPage(item.id)}
              >
                <Icon size={18} />
                <span>{item.label}</span>
                {item.id === "devices" && discoveredPeers.length > 0 && (
                  <span className="nav-count">{discoveredPeers.length}</span>
                )}
              </button>
            );
          })}
        </nav>

        <div className="sidebar__bottom">
          <div className="local-device">
            <span className="local-device__icon"><Cpu size={16} /></span>
            <div>
              <strong>{snapshot.localDevice.name}</strong>
              <span>{snapshot.localDevice.version}</span>
            </div>
          </div>
          <div className="sidebar__version">
            <span className={`presence ${serviceIsOn ? "presence--connected" : "presence--offline"}`} />
            {api.isTauriRuntime ? "本机服务 · v0.1.0" : "浏览器演示模式"}
          </div>
        </div>
      </aside>

      <div className="app-main">
        <header className="topbar">
          <div className="mobile-brand">
            <BrandMark /> <strong>InputMesh</strong>
          </div>
          <div className="topbar__context">
            <span>{navItems.find((item) => item.id === page)?.label}</span>
            {api.isTauriRuntime ? (
              <span className="runtime-badge"><ShieldCheck size={12} /> 安全直连</span>
            ) : (
              <span className="runtime-badge runtime-badge--demo"><Sparkles size={12} /> 可交互演示</span>
            )}
          </div>

          <div className="service-control">
            <div className="service-control__label">
              <span
                className={`status-light status-light--${snapshot.serviceStatus}`}
                aria-hidden="true"
              />
              <span>
                <strong>{serviceLabel}</strong>
                <small>{connectedPeers.length} 台设备在线</small>
              </span>
            </div>
            <Toggle
              checked={serviceIsOn}
              disabled={busy === "sharing" || snapshot.serviceStatus === "starting"}
              label="键鼠共享服务"
              onChange={(enabled) =>
                run("sharing", () => api.setSharingEnabled(enabled))
              }
            />
          </div>
        </header>

        <nav className="mobile-nav" aria-label="移动端主导航">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={page === item.id ? "is-active" : ""}
                onClick={() => setPage(item.id)}
              >
                <Icon size={17} /> {item.label}
              </button>
            );
          })}
        </nav>

        <main className="content">
          {error && (
            <div className="error-toast" role="alert">
              <AlertCircle size={17} />
              <span>{error}</span>
              <button type="button" aria-label="关闭错误提示" onClick={() => setError(null)}>
                <X size={15} />
              </button>
            </div>
          )}

          {snapshot.permission.requiresAction && (
            <PermissionBanner
              snapshot={snapshot}
              pending={busy === "permission"}
              onOpen={() => run("permission", api.openPermissionSettings)}
            />
          )}

          {page === "layout" && (
            <LayoutPage
              snapshot={snapshot}
              onOpenDevices={() => setPage("devices")}
              onMoveScreen={(screenId, x, y) =>
                run(`screen-${screenId}`, () => api.updateScreenPosition(screenId, x, y))
              }
              onScreenEnabled={(screenId, enabled) =>
                run(`screen-${screenId}`, () => api.setScreenEnabled(screenId, enabled))
              }
            />
          )}

          {page === "devices" && (
            <PageSection
              eyebrow="安全连接"
              title="局域网设备"
              description="自动发现同一网络中的 InputMesh 设备，并通过六位数字完成可信配对。"
            >
              <DevicePanel
                peers={snapshot.peers}
                pairingCode={activePairingCode}
                onPair={(peerId) => run(`peer-${peerId}`, () => api.pairPeer(peerId))}
                onReject={(peerId) => run(`peer-${peerId}`, () => api.rejectPeer(peerId))}
                onRefresh={() => run("refresh", api.refreshDiscovery)}
              />
            </PageSection>
          )}

          {page === "settings" && (
            <PageSection
              eyebrow="偏好设置"
              title="让跨屏切换更顺手"
              description="调整边缘触发手感和系统启动行为，这些设置会同步到本机服务。"
            >
              <SettingsPanel
                settings={snapshot.settings}
                onSave={(settings) => run("settings", () => api.updateSettings(settings))}
              />
            </PageSection>
          )}

          {page === "logs" && <LogPage logs={snapshot.logs} />}
        </main>
      </div>
    </div>
  );
}

function LayoutPage({
  snapshot,
  onOpenDevices,
  onMoveScreen,
  onScreenEnabled,
}: {
  snapshot: AppSnapshot;
  onOpenDevices: () => void;
  onMoveScreen: (screenId: string, x: number, y: number) => void;
  onScreenEnabled: (screenId: string, enabled: boolean) => void;
}) {
  const connected = snapshot.peers.filter((peer) => peer.status === "connected");
  const activeScreen = snapshot.screens.find((screen) => screen.id === snapshot.activeScreenId);

  return (
    <>
      <section className="welcome-row">
        <div>
          <span className="eyebrow">控制台</span>
          <h1>一套键鼠，穿梭所有屏幕</h1>
          <p>
            将屏幕拖到它们在桌面上的真实位置。指针越过相邻边缘时，控制会自然切换。
          </p>
        </div>
        <div className="quick-stats">
          <div>
            <span><Monitor size={15} /> 可用屏幕</span>
            <strong>{snapshot.screens.filter((screen) => screen.online).length}</strong>
          </div>
          <div>
            <span><Wifi size={15} /> 已连接设备</span>
            <strong>{connected.length}</strong>
          </div>
          <div>
            <span><MousePointer2 size={15} /> 当前控制</span>
            <strong>{activeScreen?.ownerName ?? "本机"}</strong>
          </div>
        </div>
      </section>

      <section className="topology-panel panel-card">
        <div className="panel-card__heading topology-panel__heading">
          <div>
            <span className="eyebrow">工作区</span>
            <h2>屏幕布局</h2>
          </div>
          <div className="topology-legend">
            <span><i className="legend-dot legend-dot--active" />当前屏幕</span>
            <span><i className="legend-dot" />已启用</span>
            <span><i className="legend-dot legend-dot--disabled" />未启用</span>
          </div>
        </div>
        <ScreenTopology
          screens={snapshot.screens}
          localDeviceId={snapshot.localDevice.id}
          activeScreenId={snapshot.activeScreenId}
          onMove={onMoveScreen}
          onEnabledChange={onScreenEnabled}
        />
        <div className="topology-panel__footer">
          <span>
            <Keyboard size={14} /> 聚焦屏幕后可用方向键微调，按住 Shift 每次移动 100 px
          </span>
          <span>布局会自动保存</span>
        </div>
      </section>

      <section className="dashboard-grid">
        <article className="panel-card connection-summary">
          <div className="panel-card__heading">
            <div>
              <span className="eyebrow">连接</span>
              <h2>可信设备</h2>
            </div>
            <button type="button" className="text-button" onClick={onOpenDevices}>
              管理设备 <ChevronRight size={14} />
            </button>
          </div>
          {connected.length > 0 ? (
            <div className="trusted-devices">
              {connected.slice(0, 3).map((peer) => (
                <div className="trusted-device" key={peer.id}>
                  <span className="trusted-device__icon"><Monitor size={17} /></span>
                  <div>
                    <strong>{peer.name}</strong>
                    <span>{peer.address ?? "安全直连"} · {peer.latencyMs ?? "—"} ms</span>
                  </div>
                  <Check size={15} />
                </div>
              ))}
            </div>
          ) : (
            <div className="mini-empty">暂时没有已连接设备</div>
          )}
        </article>

        <article className="panel-card network-summary">
          <div className="panel-card__heading">
            <div>
              <span className="eyebrow">网络</span>
              <h2>本机服务</h2>
            </div>
            <Radio size={19} />
          </div>
          <dl className="detail-list">
            <div><dt>监听端口</dt><dd>{snapshot.listenPort ?? "自动"}</dd></div>
            <div><dt>传输通道</dt><dd><ShieldCheck size={13} /> 加密直连</dd></div>
            <div>
              <dt>设备发现</dt>
              <dd>
                <span className={snapshot.serviceStatus === "running" ? "live-dot" : ""} />
                {snapshot.serviceStatus === "running" ? "正在扫描" : "暂不可用"}
              </dd>
            </div>
          </dl>
        </article>
      </section>
    </>
  );
}

function PermissionBanner({
  snapshot,
  pending,
  onOpen,
}: {
  snapshot: AppSnapshot;
  pending: boolean;
  onOpen: () => void;
}) {
  return (
    <section className="permission-banner" role="alert">
      <span className="permission-banner__icon"><ShieldAlert size={20} /></span>
      <div>
        <strong>还差一项系统权限</strong>
        <p>{snapshot.permission.helpText}</p>
        <div className="permission-checks">
          <span className={snapshot.permission.accessibility ? "is-ready" : ""}>
            {snapshot.permission.accessibility ? <Check size={12} /> : <AlertCircle size={12} />}
            辅助功能
          </span>
          <span className={snapshot.permission.inputMonitoring ? "is-ready" : ""}>
            {snapshot.permission.inputMonitoring ? <Check size={12} /> : <AlertCircle size={12} />}
            输入监控
          </span>
        </div>
      </div>
      <button type="button" className="button button--warning" disabled={pending} onClick={onOpen}>
        {pending ? <LoaderCircle className="spin" size={15} /> : <ExternalLink size={15} />}
        打开系统设置
      </button>
    </section>
  );
}

function PageSection({
  eyebrow,
  title,
  description,
  children,
}: {
  eyebrow: string;
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="page-section">
      <div className="page-heading">
        <span className="eyebrow">{eyebrow}</span>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {children}
    </section>
  );
}

function LogPage({ logs }: { logs: ActivityLog[] }) {
  const [filter, setFilter] = useState<LogFilter>("all");
  const [copied, setCopied] = useState(false);
  const visibleLogs = useMemo(
    () => logs.filter((entry) => filter === "all" || entry.level === filter),
    [filter, logs],
  );

  const copyLogs = async () => {
    const text = visibleLogs
      .map((entry) => `${new Date(entry.timestamp).toISOString()} [${entry.level}] ${entry.message}`)
      .join("\n");
    try {
      await navigator.clipboard?.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  };

  return (
    <PageSection
      eyebrow="诊断"
      title="活动日志"
      description="查看设备发现、连接状态和跨屏控制事件。日志仅保存在本机。"
    >
      <section className="log-panel panel-card">
        <div className="log-toolbar">
          <div className="segmented-control" aria-label="筛选日志级别">
            <ListFilter size={15} />
            {(["all", "info", "warning", "error"] as LogFilter[]).map((level) => (
              <button
                type="button"
                key={level}
                className={filter === level ? "is-active" : ""}
                onClick={() => setFilter(level)}
              >
                {level === "all" ? "全部" : level === "info" ? "信息" : level === "warning" ? "提醒" : "错误"}
              </button>
            ))}
          </div>
          <button className="button button--quiet button--small" type="button" onClick={copyLogs}>
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? "已复制" : "复制日志"}
          </button>
        </div>
        <div className="log-list">
          {visibleLogs.length === 0 ? (
            <div className="empty-state">
              <CircleHelp size={24} />
              <strong>此级别暂无日志</strong>
            </div>
          ) : (
            visibleLogs.map((entry) => (
              <article className="log-row" key={entry.id}>
                <span className={`log-level log-level--${entry.level}`}>
                  {entry.level === "info" ? <Check size={12} /> : <AlertCircle size={12} />}
                </span>
                <time dateTime={new Date(entry.timestamp).toISOString()}>
                  {formatLogTime(entry.timestamp)}
                </time>
                <p>{entry.message}</p>
              </article>
            ))
          )}
        </div>
      </section>
    </PageSection>
  );
}

function BrandMark() {
  return (
    <span className="brand-mark" aria-hidden="true">
      <span />
      <ArrowRight size={13} />
      <span />
    </span>
  );
}

function formatLogTime(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(timestamp);
}

function errorMessage(reason: unknown) {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "操作未完成，请稍后重试。";
}

export default App;
