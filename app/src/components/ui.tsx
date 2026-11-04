import { useState } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";

export function cx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "primary" | "danger" | "ghost" | "success";
  busy?: boolean;
};

export function Button({
  variant = "default",
  busy = false,
  className,
  children,
  disabled,
  ...rest
}: ButtonProps) {
  return (
    <button
      className={cx("btn", `btn-${variant}`, className)}
      disabled={disabled || busy}
      {...rest}
    >
      {busy && <span className="spinner" />}
      {children}
    </button>
  );
}

export function Card({
  title,
  subtitle,
  actions,
  children,
  className,
  flush,
}: {
  title?: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  flush?: boolean;
}) {
  return (
    <section className={cx("card", className)}>
      {(title || actions) && (
        <header className="card-head">
          <div>
            {title && <h2>{title}</h2>}
            {subtitle && <p>{subtitle}</p>}
          </div>
          {actions && <div className="card-actions">{actions}</div>}
        </header>
      )}
      <div className={cx("card-body", flush && "flush")}>{children}</div>
    </section>
  );
}

export function Badge({
  tone = "muted",
  children,
}: {
  tone?: "muted" | "green" | "red" | "amber" | "accent" | "blue";
  children: ReactNode;
}) {
  return <span className={cx("badge", `badge-${tone}`)}>{children}</span>;
}

export function Field({
  label,
  hint,
  children,
  inline,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  inline?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const text = hint?.trim() || `Impostazione “${label}” letta dal file di configurazione.`;

  return (
    <label className={cx("field", inline && "field-inline")}>
      <span className="field-label">
        {label}
        <button
          type="button"
          className={cx("field-hint", open && "open")}
          title={text}
          aria-label={`Spiegazione di ${label}`}
          aria-expanded={open}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            setOpen((value) => !value);
          }}
        >
          ?
        </button>
      </span>
      {open && <span className="field-help">{text}</span>}
      {children}
    </label>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  ariaLabel,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label?: string;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      className={cx("toggle", checked && "on")}
      onClick={() => onChange(!checked)}
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
    >
      <span className="toggle-track"><span className="toggle-thumb" /></span>
      {label && <span className="toggle-label">{label}</span>}
    </button>
  );
}

export function Stat({
  label,
  value,
  hint,
  tone,
}: {
  label: string;
  value: ReactNode;
  hint?: string;
  tone?: "green" | "red" | "accent";
}) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className={cx("stat-value", tone && `text-${tone}`)}>{value}</span>
      {hint && <span className="stat-hint">{hint}</span>}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}

export function Spinner() {
  return <span className="spinner" />;
}
