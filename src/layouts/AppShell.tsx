import { useEffect, useRef, useState } from "react";
import {
  Ban,
  Car,
  ChartColumn,
  ChevronsLeft,
  ChevronsRight,
  Database,
  History,
  KeyRound,
  LayoutDashboard,
  Clock,
  LogOut,
  Map,
  Radio,
  RefreshCw,
  ScrollText,
  ScanEye,
  Settings,
  ShieldAlert,
  Siren,
  SquareTerminal,
  UserCog,
  Users,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { formatRelative } from "@/lib/format";
import { ROUTES, ROUTE_TITLES, navigate, useRoute, type RouteKey } from "@/lib/router";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useToast } from "@/state/toast";
import { IconButton } from "@/components/ui/Button";
import { StatusDot } from "@/components/ui/Badge";
import { NoAccess } from "@/components/PermissionGate";
import AuthPage from "@/pages/Auth";
import DashboardPage from "@/pages/Dashboard";
import DispatchPage from "@/pages/Dispatch";
import BolosPage from "@/pages/Bolos";
import DutyRosterPage from "@/pages/DutyRoster";
import StatisticsPage from "@/pages/Statistics";
import LiveMapPage from "@/pages/LiveMap";
import PlayersPage from "@/pages/Players";
import PlayerDatabasePage from "@/pages/PlayerDatabase";
import ModerationPage from "@/pages/Moderation";
import BannedPlayersPage from "@/pages/BannedPlayers";
import ServerCommandsPage from "@/pages/ServerCommands";
import VehiclesPage from "@/pages/Vehicles";
import EmergencyCallsPage from "@/pages/EmergencyCalls";
import ServerLogsPage from "@/pages/ServerLogs";
import StaffManagementPage from "@/pages/StaffManagement";
import AuditLogsPage from "@/pages/AuditLogs";
import ApiConfigPage from "@/pages/ApiConfig";
import SettingsPage from "@/pages/Settings";

interface NavItem {
  key: RouteKey;
  label: string;
  icon: LucideIcon;
  permission: string;
}

const NAV_GROUPS: { title: string; items: NavItem[] }[] = [
  {
    title: "Operations",
    items: [
      { key: "dashboard", label: "Dashboard", icon: LayoutDashboard, permission: "dashboard.view" },
      { key: "dispatch", label: "CAD Dispatch", icon: Radio, permission: "dispatch.view" },
      { key: "bolos", label: "BOLO Board", icon: ScanEye, permission: "bolos.view" },
      { key: "map", label: "Live Map", icon: Map, permission: "map.view" },
      { key: "players", label: "Players", icon: Users, permission: "players.view" },
      { key: "database", label: "Player Database", icon: Database, permission: "players.view" },
      { key: "moderation", label: "Moderation", icon: ShieldAlert, permission: "moderation.view" },
      { key: "bans", label: "Banned Players", icon: Ban, permission: "moderation.view" },
      { key: "roster", label: "Duty Roster", icon: Clock, permission: "shifts.view" },
      { key: "stats", label: "Statistics", icon: ChartColumn, permission: "stats.view" },
    ],
  },
  {
    title: "Server",
    items: [
      { key: "commands", label: "Server Commands", icon: SquareTerminal, permission: "commands.view" },
      { key: "vehicles", label: "Vehicles", icon: Car, permission: "vehicles.view" },
      { key: "emergency", label: "Emergency Calls", icon: Siren, permission: "emergency.view" },
      { key: "logs", label: "Server Logs", icon: ScrollText, permission: "logs.view" },
    ],
  },
  {
    title: "Administration",
    items: [
      { key: "staff", label: "Staff Management", icon: UserCog, permission: "staff.view" },
      { key: "audit", label: "Audit Logs", icon: History, permission: "audit.view" },
      { key: "api", label: "API Configuration", icon: KeyRound, permission: "api.configure" },
      { key: "settings", label: "Settings", icon: Settings, permission: "settings.manage" },
    ],
  },
];

const PAGE_PERMISSIONS: Record<RouteKey, string> = {
  dashboard: "dashboard.view",
  dispatch: "dispatch.view",
  bolos: "bolos.view",
  roster: "shifts.view",
  stats: "stats.view",
  map: "map.view",
  players: "players.view",
  database: "players.view",
  moderation: "moderation.view",
  bans: "moderation.view",
  commands: "commands.view",
  vehicles: "vehicles.view",
  emergency: "emergency.view",
  logs: "logs.view",
  staff: "staff.view",
  audit: "audit.view",
  api: "api.configure",
  settings: "settings.manage",
};

function renderPage(route: RouteKey) {
  switch (route) {
    case "dashboard":
      return <DashboardPage />;
    case "dispatch":
      return <DispatchPage />;
    case "bolos":
      return <BolosPage />;
    case "roster":
      return <DutyRosterPage />;
    case "stats":
      return <StatisticsPage />;
    case "map":
      return <LiveMapPage />;
    case "players":
      return <PlayersPage />;
    case "database":
      return <PlayerDatabasePage />;
    case "moderation":
      return <ModerationPage />;
    case "bans":
      return <BannedPlayersPage />;
    case "commands":
      return <ServerCommandsPage />;
    case "vehicles":
      return <VehiclesPage />;
    case "emergency":
      return <EmergencyCallsPage />;
    case "logs":
      return <ServerLogsPage />;
    case "staff":
      return <StaffManagementPage />;
    case "audit":
      return <AuditLogsPage />;
    case "api":
      return <ApiConfigPage />;
    case "settings":
      return <SettingsPage />;
    default:
      return <DashboardPage />;
  }
}

const CONNECTION_LABEL: Record<string, string> = {
  connected: "Connected",
  error: "Connection issue",
  not_configured: "Not configured",
};

export default function AppShell() {
  const { session, has, signOut } = useSession();
  const { snapshot, settings, refreshing, refresh, lastSync } = useSnapshot();
  const route = useRoute();
  const startupApplied = useRef(false);
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem("avero-cad.sidebar") === "collapsed";
    } catch {
      return false;
    }
  });
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    try {
      window.localStorage.setItem("avero-cad.sidebar", collapsed ? "collapsed" : "expanded");
    } catch {
      /* ignore storage failures */
    }
  }, [collapsed]);

  // Apply the configured start page once, and only when no route was requested.
  useEffect(() => {
    if (startupApplied.current || !settings) return;
    startupApplied.current = true;
    if (window.location.hash) return;
    const target = settings.startupPage as RouteKey;
    if ((ROUTES as readonly string[]).includes(target) && has(PAGE_PERMISSIONS[target])) {
      navigate(target);
    }
  }, [settings, has]);

  useEffect(() => {
    if (!menuOpen) return;
    const onClick = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", onClick);
    return () => window.removeEventListener("mousedown", onClick);
  }, [menuOpen]);

  const toast = useToast();

  if (!session) {
    return <AuthPage />;
  }

  const connectionState = snapshot?.connection.state ?? "not_configured";
  const allowed = has(PAGE_PERMISSIONS[route]);

  return (
    <div className="flex h-full min-h-0 bg-canvas">
      <aside
        className={cn(
          "flex shrink-0 flex-col border-r border-line bg-surface transition-[width] duration-150",
          collapsed ? "w-[60px]" : "w-[228px]",
        )}
      >
        <div
          className={cn(
            "flex h-14 items-center border-b border-line",
            collapsed ? "justify-center px-2" : "gap-2.5 px-4",
          )}
        >
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded border border-line-strong bg-surface-3 text-[11px] font-bold tracking-tight text-ink">
            AC
          </div>
          {!collapsed ? (
            <div className="min-w-0">
              <p className="truncate text-[13px] font-semibold tracking-tight text-ink">Avero CAD</p>
              <p className="truncate text-[10px] uppercase tracking-[0.12em] text-faint">
                Dispatch console
              </p>
            </div>
          ) : null}
        </div>

        <nav className="min-h-0 flex-1 overflow-y-auto py-2">
          {NAV_GROUPS.map((group) => {
            const visible = group.items.filter((item) => has(item.permission));
            if (visible.length === 0) return null;
            return (
              <div key={group.title} className="mb-3">
                {!collapsed ? (
                  <p className="px-4 pb-1.5 text-[10px] font-medium uppercase tracking-[0.12em] text-faint">
                    {group.title}
                  </p>
                ) : (
                  <div className="mx-3 mb-1.5 h-px bg-line" />
                )}
                <ul className="space-y-0.5 px-2">
                  {visible.map((item) => {
                    const active = route === item.key;
                    const Icon = item.icon;
                    return (
                      <li key={item.key}>
                        <button
                          type="button"
                          onClick={() => navigate(item.key)}
                          title={collapsed ? item.label : undefined}
                          className={cn(
                            "flex w-full items-center rounded-md text-[13px] transition-colors",
                            collapsed ? "justify-center px-0 py-2" : "gap-2.5 px-2.5 py-[7px]",
                            active
                              ? "bg-surface-3 text-ink"
                              : "text-muted hover:bg-surface-2 hover:text-ink",
                          )}
                          aria-current={active ? "page" : undefined}
                        >
                          <Icon size={15} className={active ? "text-white" : "text-faint"} />
                          {!collapsed ? <span className="truncate">{item.label}</span> : null}
                          {!collapsed && active ? (
                            <span className="ml-auto h-3.5 w-px bg-white/60" />
                          ) : null}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </div>
            );
          })}
        </nav>

        <div className="border-t border-line p-2">
          <button
            type="button"
            onClick={() => setCollapsed((value) => !value)}
            className={cn(
              "flex w-full items-center rounded-md py-1.5 text-[12px] text-faint transition-colors hover:bg-surface-2 hover:text-ink",
              collapsed ? "justify-center" : "gap-2 px-2",
            )}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {collapsed ? <ChevronsRight size={14} /> : <ChevronsLeft size={14} />}
            {!collapsed ? <span>Collapse</span> : null}
          </button>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b border-line bg-surface px-4">
          <div className="flex min-w-0 items-center gap-3">
            <h1 className="truncate text-[13px] font-semibold tracking-tight text-ink">
              {ROUTE_TITLES[route]}
            </h1>
            <span className="hidden items-center gap-1.5 rounded border border-line bg-surface-2 px-2 py-0.5 text-[11px] text-muted md:inline-flex">
              <StatusDot
                tone={
                  connectionState === "connected"
                    ? "online"
                    : connectionState === "error"
                      ? "offline"
                      : "neutral"
                }
                pulse={connectionState === "connected"}
              />
              {CONNECTION_LABEL[connectionState] ?? connectionState}
              {snapshot?.connection.latencyMs ? (
                <span className="text-faint">· {snapshot.connection.latencyMs} ms</span>
              ) : null}
            </span>
            {lastSync ? (
              <span className="hidden text-[11px] text-faint lg:inline">
                Synced {formatRelative(lastSync)}
              </span>
            ) : null}
          </div>

          <div className="flex shrink-0 items-center gap-1.5">
            <IconButton
              label="Refresh live data"
              onClick={() => {
                refresh(true).catch(() => undefined);
                toast.info("Refreshing live server data");
              }}
              disabled={refreshing || !has("dashboard.view")}
            >
              <RefreshCw size={14} className={refreshing ? "animate-spin" : undefined} />
            </IconButton>

            <div className="relative" ref={menuRef}>
              <button
                type="button"
                onClick={() => setMenuOpen((value) => !value)}
                className="flex items-center gap-2 rounded-md border border-line bg-surface-2 py-1 pl-1 pr-2.5 text-left transition-colors hover:border-line-strong"
                aria-haspopup="menu"
                aria-expanded={menuOpen}
              >
                <span className="flex h-6 w-6 items-center justify-center rounded border border-line bg-surface-3 text-[10px] font-semibold text-ink">
                  {session.user.displayName.slice(0, 2).toUpperCase()}
                </span>
                <span className="hidden min-w-0 sm:block">
                  <span className="block truncate text-[12px] font-medium leading-3.5 text-ink">
                    {session.user.displayName}
                  </span>
                  <span className="block truncate text-[10px] leading-3.5 text-faint">
                    {session.user.roleName}
                  </span>
                </span>
              </button>
              {menuOpen ? (
                <div
                  role="menu"
                  className="absolute right-0 top-[calc(100%+6px)] z-20 w-52 animate-fade-in overflow-hidden rounded-md border border-line-strong bg-surface shadow-[0_16px_40px_rgba(0,0,0,0.6)]"
                >
                  <div className="border-b border-line px-3 py-2.5">
                    <p className="truncate text-[12px] font-medium text-ink">
                      {session.user.username}
                    </p>
                    <p className="truncate text-[11px] text-faint">{session.user.roleName}</p>
                  </div>
                  {has("settings.manage") ? (
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setMenuOpen(false);
                        navigate("settings");
                      }}
                      className="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] text-muted transition-colors hover:bg-surface-2 hover:text-ink"
                    >
                      <Settings size={13} /> Settings
                    </button>
                  ) : null}
                  <button
                    type="button"
                    role="menuitem"
                    onClick={() => {
                      setMenuOpen(false);
                      signOut().catch(() => undefined);
                    }}
                    className="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] text-muted transition-colors hover:bg-surface-2 hover:text-ink"
                  >
                    <LogOut size={13} /> Sign out
                  </button>
                </div>
              ) : null}
            </div>
          </div>
        </header>

        <main className="min-h-0 flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[1560px] px-4 py-4">
            {allowed ? (
              <div key={route} className="animate-fade-in">
                {renderPage(route)}
              </div>
            ) : (
              <NoAccess />
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
