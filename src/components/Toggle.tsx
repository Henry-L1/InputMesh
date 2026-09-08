import type { InputHTMLAttributes } from "react";

interface ToggleProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "onChange"> {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  compact?: boolean;
}

export function Toggle({
  checked,
  onChange,
  label,
  compact = false,
  className = "",
  ...props
}: ToggleProps) {
  return (
    <label
      className={`toggle ${compact ? "toggle--compact" : ""} ${className}`.trim()}
      title={label}
    >
      <input
        {...props}
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="toggle__track" aria-hidden="true">
        <span className="toggle__thumb" />
      </span>
    </label>
  );
}
