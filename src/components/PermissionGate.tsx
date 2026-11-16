import type { ReactNode } from "react";
import { useSession } from "@/state/session";

/**
 * Hides UI a staff member cannot use. This is purely cosmetic: the Rust backend
 * re-checks every permission on every command.
 */
export function PermissionGate({
  permission,
  children,
  fallback = null,
}: {
  permission: string;
  children: ReactNode;
  fallback?: ReactNode;
}) {
  const { has } = useSession();
  return <>{has(permission) ? children : fallback}</>;
}

export function NoAccess({ message }: { message?: string }) {
  return (
    <div className="panel flex flex-col items-center justify-center px-6 py-16 text-center">
      <p className="text-[13px] font-medium text-ink">Access restricted</p>
      <p className="mt-1 max-w-sm text-xs leading-5 text-muted">
        {message ??
          "Your role does not include this module. Ask the Server Owner to grant the matching permission."}
      </p>
    </div>
  );
}
