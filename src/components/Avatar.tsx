import { useEffect, useState } from "react";
import { api, errorMessage } from "@/lib/api";
import { cn } from "@/lib/cn";
import { initials } from "@/lib/format";
import { useSession } from "@/state/session";
import type { AvatarView } from "@/types";

const cache = new Map<string, AvatarView>();

export function Avatar({
  userId,
  name,
  size = 32,
  fullBody = false,
  className,
}: {
  userId: number | null | undefined;
  name: string;
  size?: number;
  fullBody?: boolean;
  className?: string;
}) {
  const { token, has } = useSession();
  const key = userId ? `${userId}:${fullBody ? "body" : "head"}` : "";
  const [state, setState] = useState<"idle" | "loading" | "ready" | "error">(
    key && cache.has(key) ? "ready" : "idle",
  );
  const [url, setUrl] = useState<string | null>(key ? (cache.get(key)?.dataUrl ?? null) : null);

  useEffect(() => {
    if (!userId || !token || !has("players.view")) {
      setState(key && cache.has(key) ? "ready" : "error");
      return;
    }
    const cached = cache.get(key);
    if (cached) {
      setUrl(cached.dataUrl);
      setState("ready");
      return;
    }
    let cancelled = false;
    setState("loading");
    api
      .avatar(token, userId, fullBody)
      .then((view) => {
        if (cancelled) return;
        cache.set(key, view);
        setUrl(view.dataUrl);
        setState("ready");
      })
      .catch((error) => {
        if (cancelled) return;
        setState("error");
        // Roblox may not have a rendered thumbnail yet; this is not fatal.
        void errorMessage(error);
      });
    return () => {
      cancelled = true;
    };
  }, [userId, token, has, key, fullBody]);

  const dimensions = { width: size, height: size };

  if (state === "ready" && url) {
    return (
      <img
        src={url}
        alt={`${name} Roblox avatar`}
        width={size}
        height={size}
        loading="lazy"
        draggable={false}
        className={cn("shrink-0 rounded-md border border-line bg-surface-2 object-cover", className)}
        style={dimensions}
      />
    );
  }

  if (state === "loading") {
    return (
      <div
        className={cn("shrink-0 animate-pulse rounded-md border border-line bg-surface-3", className)}
        style={dimensions}
        aria-label={`Loading ${name}'s avatar`}
      />
    );
  }

  return (
    <div
      className={cn(
        "flex shrink-0 items-center justify-center rounded-md border border-line bg-surface-2 text-[10px] font-semibold uppercase tracking-wide text-faint",
        className,
      )}
      style={dimensions}
      aria-label={`${name} avatar unavailable`}
      title="Roblox avatar unavailable"
    >
      {initials(name)}
    </div>
  );
}
