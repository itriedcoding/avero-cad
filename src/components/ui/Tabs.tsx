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
              "flex items-center gap-1.5 whitespace-nowrap rounded-md px-2.5 py-1.5 text-[12px] font-medium transition-colors",
              active
                ? "bg-surface-3 text-ink"
                : "text-muted hover:bg-surface-2 hover:text-ink",
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
