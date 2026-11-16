import { useEffect, useState } from "react";

export const ROUTES = [
  "dashboard",
  "dispatch",
  "map",
  "players",
  "database",
  "moderation",
  "bans",
  "commands",
  "vehicles",
  "emergency",
  "logs",
  "staff",
  "audit",
  "api",
  "settings",
] as const;

export type RouteKey = (typeof ROUTES)[number];

export const ROUTE_TITLES: Record<RouteKey, string> = {
  dashboard: "Dashboard",
  dispatch: "CAD Dispatch",
  map: "Live Map",
  players: "Players",
  database: "Player Database",
  moderation: "Moderation",
  bans: "Banned Players",
  commands: "Server Commands",
  vehicles: "Vehicles",
  emergency: "Emergency Calls",
  logs: "Server Logs",
  staff: "Staff Management",
  audit: "Audit Logs",
  api: "API Configuration",
  settings: "Settings",
};

function currentRoute(): RouteKey {
  const raw = window.location.hash.replace(/^#\/?/, "").split("?")[0] ?? "";
  return (ROUTES as readonly string[]).includes(raw) ? (raw as RouteKey) : "dashboard";
}

export function navigate(route: RouteKey): void {
  if (window.location.hash !== `#/${route}`) {
    window.location.hash = `#/${route}`;
  }
}

export function useRoute(): RouteKey {
  const [route, setRoute] = useState<RouteKey>(() => currentRoute());

  useEffect(() => {
    const onChange = () => setRoute(currentRoute());
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  return route;
}
