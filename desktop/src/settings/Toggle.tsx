import type { ReactNode } from "react";

/** Một công tắc trong khối `.toggles`: tên, dòng giải thích, và điều khiển thêm hiện bên dưới khi cần. */
export function Toggle({
  label,
  hint,
  checked,
  disabled = false,
  onChange,
  children,
}: {
  label: string;
  hint: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  /** Điều khiển thêm (số phút, giờ...), chỉ truyền vào khi công tắc đang bật. */
  children?: ReactNode;
}) {
  return (
    <div className={disabled ? "toggle toggle--off" : "toggle"}>
      <label className="toggle__main">
        <input
          className="checkbox"
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span className="toggle__text">
          <strong>{label}</strong>
          <small>{hint}</small>
        </span>
      </label>
      {children && <div className="toggle__extra">{children}</div>}
    </div>
  );
}
