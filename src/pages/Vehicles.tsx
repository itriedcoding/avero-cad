import { useMemo, useState } from "react";
import { Car, Search } from "lucide-react";
import { safeHexColor } from "@/lib/format";
import { useSnapshot } from "@/state/snapshot";
import { Badge } from "@/components/ui/Badge";
import { Input } from "@/components/ui/Field";
import { PageHeader, Skeleton } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import type { VehicleView } from "@/types";

export default function VehiclesPage() {
  const { snapshot, loading, refresh, refreshing } = useSnapshot();
  const [filter, setFilter] = useState("");

  const vehicles = snapshot?.vehicles ?? [];
  const filtered = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    if (!needle) return vehicles;
    return vehicles.filter(
      (vehicle) =>
        vehicle.name.toLowerCase().includes(needle) ||
        vehicle.owner.toLowerCase().includes(needle) ||
        vehicle.plate.toLowerCase().includes(needle) ||
        vehicle.colorName.toLowerCase().includes(needle) ||
        (vehicle.texture ?? "").toLowerCase().includes(needle),
    );
  }, [vehicles, filter]);

  return (
    <div>
      <PageHeader
        title="Vehicles"
        description="Vehicles currently spawned in your ER:LC server, including plate, livery and colour reported by API v2. The API does not expose vehicle positions."
        actions={
          <button
            type="button"
            onClick={() => void refresh(true)}
            className="rounded border border-line bg-surface-2 px-2.5 py-1.5 text-[11px] text-muted transition-colors hover:border-line-strong hover:text-ink"
          >
            {refreshing ? "Refreshing…" : "Refresh"}
          </button>
        }
      />

      <Panel>
        <PanelHeader
          title="Spawned vehicles"
          description={`${filtered.length} of ${vehicles.length} vehicles`}
          actions={<Badge tone="neutral"><Car size={11} /> {vehicles.length}</Badge>}
        />
        <div className="border-b border-line p-3">
          <div className="relative max-w-sm">
            <Search size={14} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
            <Input
              value={filter}
              onChange={(event) => setFilter(event.target.value)}
              placeholder="Filter by model, owner, plate, colour or livery"
              className="pl-8"
              spellCheck={false}
            />
          </div>
        </div>
        {loading && vehicles.length === 0 ? (
          <div className="space-y-2 p-3">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
          </div>
        ) : (
          <DataTable<VehicleView>
            dense
            columns={[
              {
                key: "name",
                header: "Vehicle",
                render: (row) => (
                  <span className="flex items-center gap-2">
                    <span
                      className="h-3.5 w-3.5 shrink-0 rounded-sm border border-line"
                      style={{ backgroundColor: safeHexColor(row.colorHex) ?? "#1b1b20" }}
                      aria-hidden="true"
                    />
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.name}</span>
                      <span className="block truncate text-[10px] text-faint">
                        {row.texture ?? "Standard livery"}
                      </span>
                    </span>
                  </span>
                ),
              },
              {
                key: "plate",
                header: "Plate",
                render: (row) => <span className="mono text-[11px] text-muted">{row.plate || "—"}</span>,
              },
              {
                key: "owner",
                header: "Owner",
                render: (row) => <span className="text-[11px] text-muted">{row.owner || "—"}</span>,
              },
              {
                key: "color",
                header: "Colour",
                render: (row) => (
                  <span className="text-[11px] text-muted">
                    {row.colorName || "—"}
                    {row.colorHex ? (
                      <span className="mono ml-1 text-[10px] text-faint">{row.colorHex}</span>
                    ) : null}
                  </span>
                ),
              },
            ]}
            rows={filtered}
            getKey={(row) => `${row.plate}-${row.name}-${row.owner}`}
            emptyTitle={vehicles.length === 0 ? "No vehicles spawned" : "No vehicles match"}
            emptyDescription={
              vehicles.length === 0
                ? "The API reports no spawned vehicles right now, or live data is not connected yet."
                : "Adjust the filter to find a vehicle."
            }
            maxHeight="600px"
            footer={`${vehicles.length} vehicle record(s) from the live API`}
          />
        )}
      </Panel>

      <div className="mt-3">
        <Panel>
          <PanelBody className="text-[11px] leading-5 text-muted">
            Vehicle locations are not part of the ER:LC API, so no vehicle markers are drawn on the
            live map. Ownership here is the username reported by the API at spawn time.
          </PanelBody>
        </Panel>
      </div>
    </div>
  );
}
