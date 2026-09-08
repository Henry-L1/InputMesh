import {
  Laptop,
  Monitor,
  Move,
  Power,
  WifiOff,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import type { ScreenInfo } from "../types";
import { Toggle } from "./Toggle";

interface ScreenTopologyProps {
  screens: ScreenInfo[];
  localDeviceId: string;
  activeScreenId?: string;
  onMove: (screenId: string, x: number, y: number) => void;
  onEnabledChange: (screenId: string, enabled: boolean) => void;
}

interface CanvasSize {
  width: number;
  height: number;
}

interface DragState {
  screenId: string;
  pointerId: number;
  clientX: number;
  clientY: number;
  originX: number;
  originY: number;
  x: number;
  y: number;
}

const CANVAS_PADDING = 38;
const MIN_SCREEN_WIDTH = 124;

function roundToGrid(value: number, grid = 10) {
  return Math.round(value / grid) * grid;
}

export function ScreenTopology({
  screens,
  localDeviceId,
  activeScreenId,
  onMove,
  onEnabledChange,
}: ScreenTopologyProps) {
  const enabledLocalScreenCount = screens.filter(
    (screen) => screen.ownerDeviceId === localDeviceId && screen.enabled && screen.online,
  ).length;
  const canvasRef = useRef<HTMLDivElement>(null);
  const [canvasSize, setCanvasSize] = useState<CanvasSize>({
    width: 760,
    height: 330,
  });
  const [drag, setDrag] = useState<DragState | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const updateSize = () => {
      const bounds = canvas.getBoundingClientRect();
      if (bounds.width > 0 && bounds.height > 0) {
        setCanvasSize({ width: bounds.width, height: bounds.height });
      }
    };

    updateSize();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(updateSize);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (drag && !screens.some((screen) => screen.id === drag.screenId)) {
      setDrag(null);
    }
  }, [drag, screens]);

  const layout = useMemo(() => {
    if (screens.length === 0) {
      return {
        minX: 0,
        minY: 0,
        scale: 0.1,
      };
    }

    const minX = Math.min(...screens.map((screen) => screen.x));
    const minY = Math.min(...screens.map((screen) => screen.y));
    const maxX = Math.max(...screens.map((screen) => screen.x + screen.width));
    const maxY = Math.max(...screens.map((screen) => screen.y + screen.height));
    const contentWidth = Math.max(maxX - minX, 1);
    const contentHeight = Math.max(maxY - minY, 1);
    const availableWidth = Math.max(canvasSize.width - CANVAS_PADDING * 2, 280);
    const availableHeight = Math.max(canvasSize.height - CANVAS_PADDING * 2, 160);

    return {
      minX,
      minY,
      scale: Math.min(availableWidth / contentWidth, availableHeight / contentHeight, 0.16),
    };
  }, [canvasSize, screens]);

  const visualFor = (screen: ScreenInfo) => {
    const position = drag?.screenId === screen.id ? drag : screen;
    const naturalWidth = screen.width * layout.scale;
    const width = Math.max(naturalWidth, MIN_SCREEN_WIDTH);
    const ratio = screen.height / screen.width;
    const height = Math.max(width * ratio, 74);
    return {
      left: CANVAS_PADDING + (position.x - layout.minX) * layout.scale,
      top: CANVAS_PADDING + (position.y - layout.minY) * layout.scale,
      width,
      height,
    };
  };

  const beginDrag = (event: ReactPointerEvent, screen: ScreenInfo) => {
    if (event.button !== 0 || !screen.online) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, input, label")) return;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    setDrag({
      screenId: screen.id,
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
      originX: screen.x,
      originY: screen.y,
      x: screen.x,
      y: screen.y,
    });
  };

  const moveDrag = (event: ReactPointerEvent) => {
    setDrag((current) => {
      if (!current || current.pointerId !== event.pointerId) return current;
      return {
        ...current,
        x: roundToGrid(current.originX + (event.clientX - current.clientX) / layout.scale),
        y: roundToGrid(current.originY + (event.clientY - current.clientY) / layout.scale),
      };
    });
  };

  const finishDrag = (event: ReactPointerEvent) => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.currentTarget.releasePointerCapture?.(event.pointerId);
    const completed = drag;
    setDrag(null);
    if (completed.x !== completed.originX || completed.y !== completed.originY) {
      onMove(completed.screenId, completed.x, completed.y);
    }
  };

  const moveWithKeyboard = (
    event: ReactKeyboardEvent<HTMLDivElement>,
    screen: ScreenInfo,
  ) => {
    const step = event.shiftKey ? 100 : 10;
    const offsets: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const offset = offsets[event.key];
    if (!offset || !screen.online) return;
    event.preventDefault();
    onMove(screen.id, screen.x + offset[0], screen.y + offset[1]);
  };

  if (screens.length === 0) {
    return (
      <div className="topology-empty">
        <Monitor size={26} />
        <strong>还没有可用屏幕</strong>
        <span>同一局域网中的设备配对后会显示在这里。</span>
      </div>
    );
  }

  return (
    <div className="topology-canvas" ref={canvasRef} data-testid="topology-canvas">
      <div className="topology-canvas__hint">
        <Move size={13} /> 拖动屏幕以匹配桌面上的实际位置
      </div>

      <svg className="topology-links" aria-hidden="true">
        {screens.slice(1).map((screen, index) => {
          const current = visualFor(screen);
          const previous = visualFor(screens[index]);
          return (
            <line
              key={`${screens[index].id}-${screen.id}`}
              x1={previous.left + previous.width / 2}
              y1={previous.top + previous.height / 2}
              x2={current.left + current.width / 2}
              y2={current.top + current.height / 2}
            />
          );
        })}
      </svg>

      {screens.map((screen) => {
        const visual = visualFor(screen);
        const isLocal = screen.ownerDeviceId === localDeviceId;
        const isActive = screen.id === activeScreenId;
        const isDragging = drag?.screenId === screen.id;
        return (
          <div
            key={screen.id}
            className={`screen-node ${isActive ? "screen-node--active" : ""} ${
              !screen.enabled ? "screen-node--disabled" : ""
            } ${!screen.online ? "screen-node--offline" : ""} ${
              isDragging ? "screen-node--dragging" : ""
            }`}
            style={visual}
            role="group"
            tabIndex={screen.online ? 0 : -1}
            aria-label={`${screen.ownerName} 的 ${screen.name}，位置 ${screen.x}, ${screen.y}`}
            onKeyDown={(event) => moveWithKeyboard(event, screen)}
            onPointerDown={(event) => beginDrag(event, screen)}
            onPointerMove={moveDrag}
            onPointerUp={finishDrag}
            onPointerCancel={() => setDrag(null)}
          >
            <div className="screen-node__bar">
              <span className="screen-node__device-icon">
                {screen.name.toLowerCase().includes("内建") ||
                screen.name.toLowerCase().includes("built") ? (
                  <Laptop size={13} />
                ) : (
                  <Monitor size={13} />
                )}
              </span>
              <Toggle
                compact
                checked={screen.enabled}
                disabled={
                  !screen.online ||
                  (isLocal && screen.enabled && enabledLocalScreenCount <= 1)
                }
                label={`${screen.name} 输入共享`}
                onChange={(enabled) => onEnabledChange(screen.id, enabled)}
              />
            </div>
            <div className="screen-node__content">
              <strong>{screen.name}</strong>
              <span>{screen.ownerName}</span>
            </div>
            <div className="screen-node__footer">
              <span>
                {screen.width} × {screen.height}
              </span>
              {isActive ? (
                <span className="screen-node__active-label">
                  <Power size={9} /> 当前
                </span>
              ) : !screen.online ? (
                <span className="screen-node__offline-label">
                  <WifiOff size={9} /> 离线
                </span>
              ) : isLocal ? (
                <span>本机</span>
              ) : null}
            </div>
          </div>
        );
      })}
    </div>
  );
}
