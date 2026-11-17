import { cn } from "@/lib/cn";

export interface TabItem<T extends string> {
  value: T;
  label: string;
  count?: number;
}

export function Tabs<T extends string>({
  items,
  value,
  onChange,
  className,
}: {
  items: TabItem<T>[];
  value: T;
  onChange: (next: T) => void;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center gap-1 overflow-x-auto", className)}>
      {items.map((item) => {
        const active = item.value === value;
        return (
          <button
            key={item.value}
            type="button"
            onClick={() => onChange(item.value)}
            className={cn(
              "flex items-center gap-1.5 whitespace-nowrap rounded-[9px] border px-2.5 py-1.5 text-[12px] font-bold transition-colors",
              active
                ? "border-accent/45 bg-accent/12 text-ink shadow-[inset_0_1px_0_rgb(255_255_255/0.09)]"
                : "border-transparent text-muted hover:bg-white/[0.06] hover:text-ink",
            )}
            aria-current={active ? "page" : undefined}
          >
            {item.label}
            {item.count !== undefined ? (
              <span className={active ? "text-muted" : "text-faint"}>{item.count}</span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}
