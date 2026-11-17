import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { EmptyState } from "@/components/ui/Feedback";
import { useSnapshotOptional } from "@/state/snapshot";

export interface Column<T> {
  key: string;
  header: ReactNode;
  render: (row: T) => ReactNode;
  className?: string;
  width?: string;
  align?: "left" | "right" | "center";
}

interface DataTableProps<T> {
  columns: Column<T>[];
  rows: T[];
  getKey: (row: T) => string | number;
  emptyTitle: string;
  emptyDescription?: string;
  emptyAction?: ReactNode;
  onRowClick?: (row: T) => void;
  selectedKey?: string | number | null;
  maxHeight?: string;
  footer?: ReactNode;
}

export function DataTable<T>({
  columns,
  rows,
  getKey,
  emptyTitle,
  emptyDescription,
  emptyAction,
  onRowClick,
  selectedKey,
  maxHeight,
  footer,
}: DataTableProps<T>) {
  // Row density follows the global "compact tables" setting; tables render outside the
  // provider (none today) fall back to comfortable rows.
  const denseRows = useSnapshotOptional()?.settings?.compactTables ?? false;

  if (rows.length === 0) {
    return (
      <EmptyState title={emptyTitle} description={emptyDescription} action={emptyAction} compact />
    );
  }

  return (
    <div className="min-w-0">
      <div className="overflow-auto" style={maxHeight ? { maxHeight } : undefined}>
        <table className="w-full border-collapse text-left">
          <thead className="sticky top-0 z-10 bg-[#0c090b]/95 backdrop-blur-md">
            <tr>
              {columns.map((column) => (
                <th
                  key={column.key}
                  style={column.width ? { width: column.width } : undefined}
                  className={cn(
                    "whitespace-nowrap border-b border-line px-3 py-2 text-[11px] font-bold uppercase tracking-[0.09em] text-faint",
                    column.align === "right" && "text-right",
                    column.align === "center" && "text-center",
                    column.className,
                  )}
                >
                  {column.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const key = getKey(row);
              const selected = selectedKey !== undefined && selectedKey !== null && selectedKey === key;
              return (
                <tr
                  key={key}
                  onClick={onRowClick ? () => onRowClick(row) : undefined}
                  className={cn(
                    "border-b border-line/70 last:border-0",
                    onRowClick && "cursor-pointer row-hover",
                    selected && "bg-accent/[0.1]",
                  )}
                >
                  {columns.map((column) => (
                    <td
                      key={column.key}
                      className={cn(
                        "px-3 align-middle text-[13px] text-ink",
                        denseRows ? "py-1.5" : "py-2.5",
                        column.align === "right" && "text-right",
                        column.align === "center" && "text-center",
                        column.className,
                      )}
                    >
                      {column.render(row)}
                    </td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {footer ? (
        <div className="border-t border-line px-3 py-2 text-[11px] text-faint">{footer}</div>
      ) : null}
    </div>
  );
}
