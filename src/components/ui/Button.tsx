import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/cn";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  leadingIcon?: ReactNode;
}

const VARIANTS: Record<Variant, string> = {
  primary:
    "border border-accent-soft/70 bg-accent text-[#0b0507] shadow-[inset_0_1px_0_0_rgb(255_255_255/0.28),0_10px_28px_-14px_rgb(255_42_58/0.9)] hover:border-accent-soft hover:bg-accent-soft active:bg-accent",
  secondary:
    "border border-white/10 bg-white/[0.045] text-ink hover:border-accent/45 hover:bg-white/[0.07] disabled:text-faint",
  ghost:
    "border border-transparent bg-transparent text-muted hover:bg-white/[0.06] hover:text-ink",
  danger:
    "border border-danger/45 bg-danger/10 text-danger hover:border-danger/70 hover:bg-danger/15 disabled:text-danger/40",
};

const SIZES: Record<Size, string> = {
  sm: "h-8 px-2.5 text-xs gap-1.5",
  md: "h-9 px-3.5 text-[13px] gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", loading, leadingIcon, className, children, disabled, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={rest.type ?? "button"}
      disabled={disabled || loading}
      className={cn(
        "inline-flex select-none items-center justify-center rounded-[10px] font-bold transition-colors duration-100",
        "focus-visible:ring-2 focus-visible:ring-accent/45",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...rest}
    >
      {loading ? <Loader2 size={14} className="animate-spin" /> : leadingIcon}
      {children}
    </button>
  );
});

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: "ghost" | "secondary";
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { label, variant = "ghost", className, children, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={rest.type ?? "button"}
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex h-8 w-8 items-center justify-center rounded-[10px] border border-transparent text-muted transition-colors",
        "hover:bg-white/[0.07] hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/40",
        variant === "secondary" && "border-white/10 bg-white/[0.05] text-ink hover:border-accent/40",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
});
