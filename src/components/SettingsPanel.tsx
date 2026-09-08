import { Gauge, Keyboard, MousePointer2, RotateCcw, Save, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import type { SharingSettings } from "../types";
import { Toggle } from "./Toggle";

interface SettingsPanelProps {
  settings: SharingSettings;
  onSave: (settings: SharingSettings) => Promise<void>;
}

const DEFAULT_SETTINGS: SharingSettings = {
  switchDelayMs: 180,
  edgeResistancePx: 12,
  takeControlOnLocalInput: true,
  launchAtLogin: false,
};

export function SettingsPanel({ settings, onSave }: SettingsPanelProps) {
  const [draft, setDraft] = useState(settings);
  const [saving, setSaving] = useState(false);

  useEffect(() => setDraft(settings), [settings]);

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  const save = async () => {
    setSaving(true);
    try {
      await onSave(draft);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="settings-layout">
      <section className="settings-card panel-card">
        <div className="panel-card__heading">
          <div>
            <span className="eyebrow">键鼠切换</span>
            <h2>手感与响应</h2>
          </div>
          <Gauge size={20} />
        </div>

        <div className="range-setting">
          <div className="range-setting__heading">
            <span className="setting-icon"><Zap size={17} /></span>
            <div>
              <label htmlFor="switch-delay">跨屏延迟</label>
              <p>指针停留在屏幕边缘多久后切换设备。</p>
            </div>
            <output htmlFor="switch-delay">{draft.switchDelayMs} ms</output>
          </div>
          <input
            id="switch-delay"
            type="range"
            min="0"
            max="800"
            step="20"
            value={draft.switchDelayMs}
            onChange={(event) =>
              setDraft((current) => ({ ...current, switchDelayMs: Number(event.target.value) }))
            }
            style={{ "--range-progress": `${(draft.switchDelayMs / 800) * 100}%` } as React.CSSProperties}
          />
          <div className="range-setting__labels"><span>立即</span><span>800 ms</span></div>
        </div>

        <div className="range-setting">
          <div className="range-setting__heading">
            <span className="setting-icon"><MousePointer2 size={17} /></span>
            <div>
              <label htmlFor="edge-resistance">边缘阻尼</label>
              <p>避免指针无意间穿过屏幕边缘。</p>
            </div>
            <output htmlFor="edge-resistance">{draft.edgeResistancePx} px</output>
          </div>
          <input
            id="edge-resistance"
            type="range"
            min="0"
            max="60"
            step="2"
            value={draft.edgeResistancePx}
            onChange={(event) =>
              setDraft((current) => ({ ...current, edgeResistancePx: Number(event.target.value) }))
            }
            style={{ "--range-progress": `${(draft.edgeResistancePx / 60) * 100}%` } as React.CSSProperties}
          />
          <div className="range-setting__labels"><span>无阻尼</span><span>强阻尼</span></div>
        </div>
      </section>

      <section className="settings-card panel-card">
        <div className="panel-card__heading">
          <div>
            <span className="eyebrow">行为</span>
            <h2>控制与启动</h2>
          </div>
          <Keyboard size={20} />
        </div>

        <div className="toggle-setting">
          <div>
            <strong>本机输入优先</strong>
            <p>检测到目标电脑本地键鼠操作时，立即交还控制。</p>
          </div>
          <Toggle
            checked={draft.takeControlOnLocalInput}
            label="本机输入优先"
            onChange={(checked) =>
              setDraft((current) => ({ ...current, takeControlOnLocalInput: checked }))
            }
          />
        </div>
        <div className="toggle-setting">
          <div>
            <strong>登录后自动启动</strong>
            <p>签名安装包完成后启用；当前开发版暂不写入系统启动项。</p>
          </div>
          <Toggle
            checked={draft.launchAtLogin}
            label="登录后自动启动"
            disabled
            onChange={() => undefined}
          />
        </div>
      </section>

      <div className="settings-actions">
        <button
          type="button"
          className="button button--quiet"
          onClick={() => setDraft(DEFAULT_SETTINGS)}
        >
          <RotateCcw size={15} /> 恢复默认
        </button>
        <span>{dirty ? "有尚未保存的更改" : "设置已同步"}</span>
        <button
          type="button"
          className="button button--primary"
          disabled={!dirty || saving}
          onClick={save}
        >
          <Save size={15} /> {saving ? "正在保存…" : "保存设置"}
        </button>
      </div>
    </div>
  );
}
