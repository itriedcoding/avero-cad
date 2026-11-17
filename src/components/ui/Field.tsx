import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { cn } from "@/lib/cn";

export function FieldLabel({
  children,
  htmlFor,
  hint,
}: {
  children: ReactNode;
  htmlFor?: string;
  hint?: string;
}) {
  return (
    <div className="mb-1.5 flex items-baseline justify-between gap-3">
      <label htmlFor={htmlFor} className="label mb-0">
        {children}
      </label>
      {hint ? <span className="text-[11px] text-faint">{hint}</span> : null}
    </div>
  );
}

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string | null;
}

export function Input({ label, hint, error, className, id, ...rest }: InputProps) {
  const inputId = id ?? `field-${Math.random().toString(36).slice(2, 9)}`;
  return (
    <div>
      {label ? (
        <FieldLabel htmlFor={inputId} hint={hint}>
          {label}
        </FieldLabel>
      ) : null}
      <input
        id={inputId}
        className={cn("field", error && "border-danger/50", className)}
        {...rest}
      />
      {error ? <p className="mt-1 text-[11px] text-danger">{error}</p> : null}
    </div>
  );
}

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: string;
}

export function Textarea({ label, hint, className, id, ...rest }: TextareaProps) {
  const areaId = id ?? `textarea-${Math.random().toString(36).slice(2, 9)}`;
  return (
    <div>
      {label ? (
        <FieldLabel htmlFor={areaId} hint={hint}>
          {label}
        </FieldLabel>
      ) : null}
      <textarea id={areaId} className={cn("field field-area resize-y", className)} {...rest} />
    </div>
  );
}

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
}

export function Select({ label, hint, className, children, id, ...rest }: SelectProps) {
  const selectId = id ?? `select-${Math.random().toString(36).slice(2, 9)}`;
  return (
    <div>
      {label ? (
        <FieldLabel htmlFor={selectId} hint={hint}>
          {label}
        </FieldLabel>
      ) : null}
      <select id={selectId} className={cn("field appearance-none pr-8", className)} {...rest}>
        {children}
      </select>
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-start justify-between gap-6 py-2.5">
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-ink">{label}</p>
        {description ? <p className="mt-0.5 text-xs leading-5 text-muted">{description}</p> : null}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative mt-0.5 h-5 w-9 shrink-0 rounded-full border transition-colors",
          checked
            ? "border-accent bg-accent shadow-[0_0_16px_-2px_rgb(255_42_58/0.8)]"
            : "border-white/12 bg-white/[0.06]",
          disabled && "opacity-40",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 h-3.5 w-3.5 rounded-full transition-transform duration-150",
            checked ? "translate-x-[18px] bg-[#0b0507]" : "translate-x-0.5 bg-muted",
          )}
        />
      </button>
    </div>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: ReactNode;
  disabled?: boolean;
}) {
  return (
    <label
      className={cn(
        "flex cursor-pointer select-none items-start gap-2.5 text-[13px] text-ink",
        disabled && "cursor-not-allowed opacity-50",
      )}
    >
      <input
        type="checkbox"
        className="mt-0.5 h-3.5 w-3.5 shrink-0 cursor-pointer accent-accent"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="leading-5">{label}</span>
    </label>
  );
}
