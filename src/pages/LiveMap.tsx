import { useMemo, useRef, useState } from "react";
import { Crosshair, Minus, Plus, RotateCcw } from "lucide-react";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useToast } from "@/state/toast";
import { errorMessage } from "@/lib/api";
import { formatRelative } from "@/lib/format";
import { MAP_OPTIONS, MAP_SIZE } from "@/lib/map";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { Input, Select } from "@/components/ui/Field";
import { InlineError, PageHeader } from "@/components/ui/Feedback";
import { KeyValue, Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";


interface Selected {
  kind: "player" | "call";
  key: string;
}

export default function LiveMapPage() {
  const { snapshot, settings, saveSettings, refresh, refreshing } = useSnapshot();
  const { has } = useSession();
  const toast = useToast();
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [selected, setSelected] = useState<Selected | null>(null);
  const [query, setQuery] = useState("");
  const [scaleDraft, setScaleDraft] = useState<string | null>(null);
  const drag = useRef<{ x: number; y: number; startX: number; startY: number } | null>(null);

  const scale = settings?.mapScale ?? 3;
  const mapImage = settings?.mapImage ?? MAP_OPTIONS[0]!.url;
  const players = snapshot?.players ?? [];
  const calls = snapshot?.emergencyCalls ?? [];
  const locatedPlayers = players.filter((player) => player.location);
  const locatedCalls = calls.filter((call) => call.x !== null && call.z !== null);

  const toPosition = useMemo(
    () => (x: number, z: number) => {
      const px = MAP_SIZE / 2 + x * scale;
      const py = MAP_SIZE / 2 + z * scale;
      return {
        left: `${Math.min(100, Math.max(0, (px / MAP_SIZE) * 100))}%`,
        top: `${Math.min(100, Math.max(0, (py / MAP_SIZE) * 100))}%`,
      };
    },
    [scale],
  );

  const filteredPlayers = locatedPlayers.filter((player) => {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return (
      player.username.toLowerCase().includes(needle) ||
      (player.callsign ?? "").toLowerCase().includes(needle) ||
      (player.location?.postal ?? "").toLowerCase().includes(needle) ||
      (player.location?.street ?? "").toLowerCase().includes(needle)
    );
  });

  const selectedPlayer =
    selected?.kind === "player" ? players.find((player) => player.username === selected.key) ?? null : null;
  const selectedCall =
    selected?.kind === "call"
      ? calls.find((call) => String(call.callNumber) === selected.key) ?? null
      : null;

  async function saveMap(patch: { mapImage?: string; mapScale?: number }) {
    try {
      await saveSettings(patch);
      toast.success("Map settings saved");
      setScaleDraft(null);
    } catch (err) {
      toast.error("Could not save map settings", errorMessage(err));
    }
  }

  function onPointerDown(event: React.PointerEvent<HTMLDivElement>) {
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
    drag.current = { x: event.clientX, y: event.clientY, startX: pan.x, startY: pan.y };
  }

  function onPointerMove(event: React.PointerEvent<HTMLDivElement>) {
    if (!drag.current) return;
    setPan({
      x: drag.current.startX + (event.clientX - drag.current.x),
      y: drag.current.startY + (event.clientY - drag.current.y),
    });
  }

  function endDrag() {
    drag.current = null;
  }

  return (
    <div>
      <PageHeader
        title="Live Map"
        description="Official ER:LC map imagery with live player and emergency call positions from API v2. Coordinates use the map centre as origin, positive X to the right and positive Z downward."
        actions={
          <Button size="sm" variant="secondary" onClick={() => void refresh(true)} loading={refreshing}>
            Refresh positions
          </Button>
        }
      />

      {snapshot?.connection.state === "error" ? (
        <div className="mb-3">
          <InlineError message={snapshot.connection.detail ?? "Live data unavailable."} />
        </div>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
        <Panel>
          <PanelHeader
            title="Map"
            description={`${filteredPlayers.length} of ${players.length} players report a location · ${locatedCalls.length} emergency calls with coordinates`}
            actions={
              <div className="flex items-center gap-1">
                <IconButton label="Zoom out" onClick={() => setZoom((value) => Math.max(0.6, value - 0.2))}>
                  <Minus size={14} />
                </IconButton>
                <span className="w-10 text-center text-[11px] text-faint">{Math.round(zoom * 100)}%</span>
                <IconButton label="Zoom in" onClick={() => setZoom((value) => Math.min(4, value + 0.2))}>
                  <Plus size={14} />
                </IconButton>
                <IconButton
                  label="Reset view"
                  onClick={() => {
                    setZoom(1);
                    setPan({ x: 0, y: 0 });
                  }}
                >
                  <RotateCcw size={14} />
                </IconButton>
              </div>
            }
          />
          <PanelBody className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <Input
                aria-label="Filter map markers"
                placeholder="Filter by username, callsign, postal or street"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                className="h-8 max-w-xs text-[12px]"
              />
              <span className="text-[11px] text-faint">
                Drag to pan. Positions only appear when the API reports them.
              </span>
            </div>

            <div
              className="relative w-full cursor-grab touch-none select-none overflow-hidden rounded-md border border-line/80 bg-[#070508] active:cursor-grabbing"
              style={{ aspectRatio: "1 / 1" }}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={endDrag}
              onPointerLeave={endDrag}
            >
              <div
                className="absolute inset-0 origin-center"
                style={{ transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)` }}
              >
                <img
                  src={mapImage}
                  alt="Official Emergency Response: Liberty County map"
                  className="absolute inset-0 h-full w-full object-contain opacity-80"
                  draggable={false}
                />
                {locatedCalls.map((call) => {
                  const key = String(call.callNumber);
                  const active = selected?.kind === "call" && selected.key === key;
                  return (
                    <button
                      key={`call-${key}`}
                      type="button"
                      onClick={() => setSelected({ kind: "call", key })}
                      style={toPosition(call.x ?? 0, call.z ?? 0)}
                      className={`absolute -translate-x-1/2 -translate-y-1/2 rounded-sm border text-[9px] font-semibold transition-colors ${
                        active
                          ? "border-accent bg-accent px-1 text-[#0b0507] shadow-[0_0_14px_rgb(255_42_58/0.8)]"
                          : "border-accent/50 bg-black/70 px-1 text-accent-soft hover:bg-accent hover:text-[#0b0507]"
                      }`}
                      title={`Emergency call #${call.callNumber} — ${call.description}`}
                    >
                      #{call.callNumber}
                    </button>
                  );
                })}
                {filteredPlayers.map((player) => {
                  const location = player.location;
                  if (!location) return null;
                  const active = selected?.kind === "player" && selected.key === player.username;
                  return (
                    <button
                      key={`player-${player.username}`}
                      type="button"
                      onClick={() => setSelected({ kind: "player", key: player.username })}
                      style={toPosition(location.x, location.z)}
                      className={`absolute h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border transition-transform ${
                        active
                          ? "border-accent bg-accent ring-2 ring-accent/35 shadow-[0_0_12px_rgb(255_42_58/0.9)]"
                          : "border-accent/55 bg-accent/25 hover:bg-accent"
                      }`}
                      title={`${player.username} · ${player.team}${location.postal ? ` · ${location.postal}` : ""}`}
                      aria-label={`${player.username} position`}
                    />
                  );
                })}
              </div>
              <div className="pointer-events-none absolute bottom-2 left-2 rounded border border-line bg-black/70 px-2 py-1 text-[10px] text-muted">
                {locatedPlayers.length === 0
                  ? "No player locations reported by the API right now"
                  : "Players: circle · Emergency calls: number tag"}
              </div>
            </div>
          </PanelBody>
        </Panel>

        <div className="space-y-4">
          <Panel>
            <PanelHeader
              title="Selection"
              description={selected ? undefined : "Select a marker on the map or a row below."}
            />
            <PanelBody>
              {selectedPlayer ? (
                <div className="space-y-1">
                  <div className="mb-2 flex items-center gap-2">
                    <h3 className="truncate text-[14px] font-semibold text-ink">{selectedPlayer.username}</h3>
                    <Badge tone="positive" dot>
                      Live
                    </Badge>
                  </div>
                  <KeyValue label="Team" value={selectedPlayer.team} />
                  <KeyValue label="Callsign" value={selectedPlayer.callsign ?? "—"} mono />
                  <KeyValue label="Permission" value={selectedPlayer.permission} />
                  <KeyValue
                    label="Postal"
                    value={selectedPlayer.location?.postal ?? "—"}
                    mono
                  />
                  <KeyValue label="Street" value={selectedPlayer.location?.street ?? "—"} />
                  <KeyValue label="Building" value={selectedPlayer.location?.building ?? "—"} />
                  <KeyValue
                    label="Coordinates"
                    value={
                      selectedPlayer.location
                        ? `${selectedPlayer.location.x.toFixed(1)}, ${selectedPlayer.location.z.toFixed(1)}`
                        : "—"
                    }
                    mono
                  />
                </div>
              ) : selectedCall ? (
                <div className="space-y-1">
                  <div className="mb-2 flex items-center gap-2">
                    <h3 className="text-[14px] font-semibold text-ink">Call #{selectedCall.callNumber}</h3>
                    <Badge tone="neutral">{selectedCall.team}</Badge>
                  </div>
                  <KeyValue label="Description" value={selectedCall.description || "—"} />
                  <KeyValue label="Location" value={selectedCall.positionDescriptor || "—"} />
                  <KeyValue
                    label="Coordinates"
                    value={
                      selectedCall.x !== null && selectedCall.z !== null
                        ? `${selectedCall.x.toFixed(1)}, ${selectedCall.z.toFixed(1)}`
                        : "—"
                    }
                    mono
                  />
                  <KeyValue label="Started" value={formatRelative(selectedCall.startedAt)} />
                </div>
              ) : (
                <p className="py-4 text-center text-xs text-faint">Nothing selected.</p>
              )}
            </PanelBody>
          </Panel>

          <Panel>
            <PanelHeader
              title="Map calibration"
              description="ER:LC publishes the 5355 × 5355 map size and axis directions, but not the world-unit to pixel scale."
            />
            <PanelBody className="space-y-3">
              {has("settings.manage") ? (
                <>
                  <Select
                    label="Map image"
                    value={mapImage}
                    onChange={(event) => void saveMap({ mapImage: event.target.value })}
                  >
                    {MAP_OPTIONS.map((option) => (
                      <option key={option.url} value={option.url}>
                        {option.label}
                      </option>
                    ))}
                  </Select>
                  <div className="flex items-end gap-2">
                    <Input
                      label="Pixels per world unit"
                      type="number"
                      step="0.1"
                      min="0.2"
                      max="20"
                      value={scaleDraft ?? String(scale)}
                      onChange={(event) => setScaleDraft(event.target.value)}
                      hint="0.2 – 20"
                    />
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={scaleDraft === null || Number(scaleDraft) === scale}
                      onClick={() => {
                        const next = Number(scaleDraft);
                        if (!Number.isFinite(next) || next < 0.2 || next > 20) {
                          toast.error("Enter a scale between 0.2 and 20");
                          return;
                        }
                        void saveMap({ mapScale: next });
                      }}
                    >
                      Apply
                    </Button>
                  </div>
                </>
              ) : (
                <KeyValue label="Pixels per world unit" value={scale} mono />
              )}
              <p className="rounded border border-line bg-surface-2 px-2.5 py-2 text-[11px] leading-5 text-muted">
                Adjust the scale until player markers line up with the postal code on the map image,
                then compare against the postal shown in the info panel. The value is stored locally
                and applies to both player and emergency call markers.
              </p>
            </PanelBody>
          </Panel>

          <Panel>
            <PanelHeader
              title="Located players"
              description="Players currently reporting coordinates."
            />
            <PanelBody padded={false}>
              {filteredPlayers.length === 0 ? (
                <p className="px-4 py-6 text-center text-xs text-faint">
                  No player locations match this filter.
                </p>
              ) : (
                <ul className="max-h-72 divide-y divide-line/70 overflow-y-auto">
                  {filteredPlayers.map((player) => (
                    <li key={player.username}>
                      <button
                        type="button"
                        onClick={() => setSelected({ kind: "player", key: player.username })}
                        className={`flex w-full items-center justify-between gap-2 px-3.5 py-2 text-left transition-colors ${
                          selected?.kind === "player" && selected.key === player.username
                            ? "bg-accent/[0.1]"
                            : "row-hover"
                        }`}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-[12px] text-ink">{player.username}</span>
                          <span className="block truncate text-[10px] text-faint">
                            {player.team}
                            {player.callsign ? ` · ${player.callsign}` : ""}
                          </span>
                        </span>
                        <span className="shrink-0 text-[10px] text-muted">
                          {player.location?.postal || "—"}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </PanelBody>
          </Panel>

          <Panel>
            <PanelHeader title="Legend" />
            <PanelBody className="space-y-2 text-[11px] text-muted">
              <p className="flex items-center gap-2">
                <Crosshair size={12} className="text-faint" /> Map origin is the centre; X grows right,
                Z grows downward.
              </p>
              <p>
                Vehicle positions are not exposed by the ER:LC API, so vehicles are listed on the
                Vehicles page instead of the map.
              </p>
              <p>
                CAD incidents are only shown on the map when their coordinates come from the live
                ER:LC emergency call feed.
              </p>
            </PanelBody>
          </Panel>
        </div>
      </div>
    </div>
  );
}
