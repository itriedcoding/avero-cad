import { useMemo, useState } from "react";
import { KeyRound, Pencil, Plus, RefreshCw, ShieldCheck, Trash2, UserPlus } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatRelative } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { Checkbox, Input, Select, Textarea } from "@/components/ui/Field";
import { ConfirmDialog, Modal } from "@/components/ui/Modal";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader, SectionTitle } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import type { PermissionInfo, RoleView, StaffUserView } from "@/types";

interface RoleDraft {
  id: number | null;
  key: string;
  name: string;
  description: string;
  permissions: string[];
  isSystem: boolean;
}

export default function StaffManagementPage() {
  const { token, isOwner, session } = useSession();
  const toast = useToast();
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createDraft, setCreateDraft] = useState({
    username: "",
    displayName: "",
    password: "",
    roleKey: "moderator",
    robloxUserId: "",
  });
  const [resetTarget, setResetTarget] = useState<StaffUserView | null>(null);
  const [resetPassword, setResetPassword] = useState("");
  const [saving, setSaving] = useState(false);
  const [roleOpen, setRoleOpen] = useState(false);
  const [roleDraft, setRoleDraft] = useState<RoleDraft>({
    id: null,
    key: "",
    name: "",
    description: "",
    permissions: [],
    isSystem: false,
  });
  const [deleteRole, setDeleteRole] = useState<RoleView | null>(null);

  const staff = useAsync<StaffUserView[]>(
    () => (token ? api.listStaff(token) : Promise.resolve([])),
    [token],
    Boolean(token),
  );
  const roles = useAsync<RoleView[]>(
    () => (token ? api.listRoles(token) : Promise.resolve([])),
    [token],
    Boolean(token),
  );
  const catalogue = useAsync<PermissionInfo[]>(
    () => (token ? api.listPermissions(token) : Promise.resolve([])),
    [token],
    Boolean(token),
  );

  const options = useMemo(() => roles.data ?? [], [roles.data]);
  const selected = useMemo(
    () => (staff.data ?? []).find((user) => user.id === selectedId) ?? (staff.data ?? [])[0] ?? null,
    [staff.data, selectedId],
  );
  const groupedPermissions = useMemo(() => {
    const map = new Map<string, PermissionInfo[]>();
    for (const permission of catalogue.data ?? []) {
      const list = map.get(permission.category) ?? [];
      list.push(permission);
      map.set(permission.category, list);
    }
    return [...map.entries()];
  }, [catalogue.data]);

  const canManage = isOwner;

  async function run(action: () => Promise<unknown>, success: string) {
    setSaving(true);
    try {
      await action();
      toast.success(success);
      await Promise.all([staff.reload(), roles.reload()]);
      return true;
    } catch (err) {
      toast.error("Action failed", errorMessage(err));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function createStaff() {
    if (!token) return;
    const userId = createDraft.robloxUserId.trim() ? Number(createDraft.robloxUserId.trim()) : null;
    if (userId !== null && (!Number.isInteger(userId) || userId <= 0)) {
      toast.error("Invalid Roblox user ID");
      return;
    }
    const ok = await run(
      () =>
        api.createStaff(
          token,
          createDraft.username.trim(),
          createDraft.displayName.trim(),
          createDraft.password,
          createDraft.roleKey,
          userId,
        ),
      "Staff account created",
    );
    if (ok) {
      setCreateOpen(false);
      setCreateDraft({ username: "", displayName: "", password: "", roleKey: "moderator", robloxUserId: "" });
    }
  }

  async function saveRole() {
    if (!token) return;
    const ok = await run(
      () =>
        roleDraft.id === null
          ? api.createRole(
              token,
              roleDraft.key.trim(),
              roleDraft.name.trim(),
              roleDraft.description.trim(),
              roleDraft.permissions,
            )
          : api.updateRoleDefinition(
              token,
              roleDraft.id,
              roleDraft.name.trim(),
              roleDraft.description.trim(),
              roleDraft.permissions,
            ),
      roleDraft.id === null ? "Role created" : "Role updated",
    );
    if (ok) setRoleOpen(false);
  }

  function toggleRolePermission(key: string, next: boolean) {
    setRoleDraft((draft) => ({
      ...draft,
      permissions: next ? [...draft.permissions, key] : draft.permissions.filter((entry) => entry !== key),
    }));
  }

  return (
    <div>
      <PageHeader
        title="Staff Management"
        description="Staff directory, role definitions and per-person permission overrides. Every change is enforced by the Rust backend and written to the audit log."
        actions={
          <>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void Promise.all([staff.reload(), roles.reload(), catalogue.reload()])}
              loading={staff.loading || roles.loading}
              leadingIcon={<RefreshCw size={13} />}
            >
              Refresh
            </Button>
            {canManage ? (
              <Button size="sm" variant="primary" onClick={() => setCreateOpen(true)} leadingIcon={<UserPlus size={13} />}>
                Add staff
              </Button>
            ) : null}
          </>
        }
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
        <Panel>
          <PanelHeader title="Staff directory" description={`${staff.data?.length ?? 0} account(s)`} />
          {staff.error ? (
            <div className="p-3">
              <InlineError message={staff.error} />
            </div>
          ) : staff.loading && !staff.data ? (
            <TableSkeleton rows={6} columns={4} />
          ) : (
            <DataTable<StaffUserView>
              dense
              onRowClick={(row) => setSelectedId(row.id)}
              selectedKey={selected?.id ?? null}
              columns={[
                {
                  key: "user",
                  header: "Account",
                  render: (row) => (
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.displayName}</span>
                      <span className="mono block truncate text-[10px] text-faint">{row.username}</span>
                    </span>
                  ),
                },
                {
                  key: "role",
                  header: "Role",
                  render: (row) => (
                    <Badge tone={row.roleKey === "owner" ? "positive" : "neutral"}>{row.roleName}</Badge>
                  ),
                },
                {
                  key: "status",
                  header: "Status",
                  render: (row) =>
                    row.isActive ? <Badge tone="muted">Active</Badge> : <Badge tone="danger">Disabled</Badge>,
                },
                {
                  key: "login",
                  header: "Last sign-in",
                  align: "right",
                  render: (row) => (
                    <span className="text-[10px] text-faint">{formatRelative(row.lastLoginAt)}</span>
                  ),
                },
              ]}
              rows={staff.data ?? []}
              getKey={(row) => row.id}
              emptyTitle="No staff accounts"
              emptyDescription="Create the first staff account to delegate moderation work."
              maxHeight="420px"
            />
          )}
        </Panel>

        <Panel>
          <PanelHeader
            title={selected ? selected.displayName : "Select a staff member"}
            description={
              selected
                ? `${selected.username} · ${selected.roleName} · created ${formatDateTime(selected.createdAt)}`
                : "Choose an account to manage its role and permissions."
            }
            actions={
              selected && canManage ? (
                <span className="flex items-center gap-1">
                  <IconButton
                    label="Reset password"
                    onClick={() => {
                      setResetTarget(selected);
                      setResetPassword("");
                    }}
                  >
                    <KeyRound size={13} />
                  </IconButton>
                </span>
              ) : null
            }
          />
          <PanelBody className="space-y-4">
            {!selected ? (
              <p className="text-[12px] text-muted">No account selected.</p>
            ) : (
              <>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Select
                    label="Role"
                    value={selected.roleKey}
                    disabled={!canManage || selected.roleKey === "owner"}
                    onChange={(event) => {
                      if (!token) return;
                      void run(
                        () => api.updateStaffRole(token, selected.id, event.target.value),
                        "Role updated",
                      );
                    }}
                  >
                    {options.map((role) => (
                      <option key={role.key} value={role.key}>
                        {role.name}
                      </option>
                    ))}
                  </Select>
                  <div>
                    <span className="label">Account state</span>
                    <div className="flex items-center gap-2">
                      <Button
                        size="sm"
                        variant={selected.isActive ? "secondary" : "primary"}
                        disabled={!canManage || selected.id === session?.user.id}
                        loading={saving}
                        onClick={() => {
                          if (!token) return;
                          void run(
                            () => api.setStaffActive(token, selected.id, !selected.isActive),
                            selected.isActive ? "Account disabled" : "Account enabled",
                          );
                        }}
                      >
                        {selected.isActive ? "Disable account" : "Enable account"}
                      </Button>
                      {selected.id === session?.user.id ? (
                        <span className="text-[10px] text-faint">This is your account</span>
                      ) : null}
                    </div>
                  </div>
                </div>

                <div>
                  <SectionTitle>Permission overrides</SectionTitle>
                  <p className="mb-2 text-[11px] leading-5 text-muted">
                    Role permissions are inherited. An override grants or denies a single permission
                    for this account without changing the role.
                  </p>
                  <div className="max-h-[320px] space-y-3 overflow-y-auto rounded border border-line bg-surface-2/40 p-2.5">
                    {groupedPermissions.length === 0 ? (
                      <p className="text-[11px] text-faint">Permission catalogue loading…</p>
                    ) : (
                      groupedPermissions.map(([category, permissions]) => (
                        <div key={category}>
                          <p className="mb-1 text-[10px] font-medium uppercase tracking-[0.08em] text-faint">
                            {category}
                          </p>
                          <div className="space-y-1">
                            {permissions.map((permission) => {
                              const override = selected.overrides.find(
                                (entry) => entry.permissionKey === permission.key,
                              );
                              const value = override ? (override.allowed ? "allow" : "deny") : "";
                              return (
                                <div
                                  key={permission.key}
                                  className="flex items-center justify-between gap-3 rounded border border-line/70 px-2 py-1"
                                >
                                  <span className="min-w-0">
                                    <span className="block truncate text-[11px] text-ink">{permission.label}</span>
                                    <span className="mono block truncate text-[9px] text-faint">
                                      {permission.key}
                                    </span>
                                  </span>
                                  <select
                                    aria-label={`${permission.label} override`}
                                    className="field h-7 w-[92px] shrink-0 px-1.5 text-[10px]"
                                    value={value}
                                    disabled={!canManage || selected.roleKey === "owner"}
                                    onChange={(event) => {
                                      if (!token) return;
                                      const next =
                                        event.target.value === "" ? null : event.target.value === "allow";
                                      void run(
                                        () =>
                                          api.setPermissionOverride(token, selected.id, permission.key, next),
                                        "Permission updated",
                                      );
                                    }}
                                  >
                                    <option value="">Inherit</option>
                                    <option value="allow">Grant</option>
                                    <option value="deny">Deny</option>
                                  </select>
                                </div>
                              );
                            })}
                          </div>
                        </div>
                      ))
                    )}
                  </div>
                </div>

                <p className="flex items-start gap-2 rounded border border-line bg-surface-2 px-3 py-2 text-[11px] leading-5 text-muted">
                  <ShieldCheck size={13} className="mt-0.5 shrink-0 text-faint" />
                  Local permissions never bypass ER:LC authorization: a granted command still has to
                  be permitted by your server key and the API.
                </p>
              </>
            )}
          </PanelBody>
        </Panel>
      </div>

      <div className="mt-5">
        <SectionTitle
          actions={
            canManage ? (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  setRoleDraft({
                    id: null,
                    key: "",
                    name: "",
                    description: "",
                    permissions: [],
                    isSystem: false,
                  });
                  setRoleOpen(true);
                }}
                leadingIcon={<Plus size={13} />}
              >
                New role
              </Button>
            ) : null
          }
        >
          Roles
        </SectionTitle>
        <Panel>
          {roles.error ? (
            <div className="p-3">
              <InlineError message={roles.error} />
            </div>
          ) : (
            <DataTable<RoleView>
              dense
              columns={[
                {
                  key: "role",
                  header: "Role",
                  render: (row) => (
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.name}</span>
                      <span className="mono block truncate text-[10px] text-faint">{row.key}</span>
                    </span>
                  ),
                },
                {
                  key: "description",
                  header: "Description",
                  render: (row) => <span className="text-[11px] text-muted">{row.description || "—"}</span>,
                },
                {
                  key: "permissions",
                  header: "Permissions",
                  render: (row) => (
                    <span className="mono text-[10px] text-muted">
                      {row.key === "owner" ? "Full access" : `${row.permissions.length} granted`}
                    </span>
                  ),
                },
                {
                  key: "users",
                  header: "Accounts",
                  render: (row) => <span className="text-[11px] text-muted">{row.userCount}</span>,
                },
                {
                  key: "actions",
                  header: "",
                  align: "right",
                  render: (row) =>
                    canManage && row.key !== "owner" ? (
                      <span className="flex justify-end gap-1">
                        <IconButton
                          label={`Edit ${row.name}`}
                          onClick={() => {
                            setRoleDraft({
                              id: row.id,
                              key: row.key,
                              name: row.name,
                              description: row.description,
                              permissions: row.permissions,
                              isSystem: row.isSystem,
                            });
                            setRoleOpen(true);
                          }}
                        >
                          <Pencil size={12} />
                        </IconButton>
                        <IconButton
                          label={`Delete ${row.name}`}
                          disabled={row.isSystem || row.userCount > 0}
                          onClick={() => setDeleteRole(row)}
                        >
                          <Trash2 size={12} />
                        </IconButton>
                      </span>
                    ) : (
                      <span className="text-[10px] text-faint">
                        {row.key === "owner" ? "Protected" : "—"}
                      </span>
                    ),
                },
              ]}
              rows={roles.data ?? []}
              getKey={(row) => row.id}
              emptyTitle="No roles"
              emptyDescription="Roles define the baseline permissions for staff accounts."
            />
          )}
        </Panel>
      </div>

      <Modal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="Add staff account"
        description="Accounts are local to this installation. Passwords are hashed with Argon2."
        footer={
          <>
            <Button variant="ghost" onClick={() => setCreateOpen(false)} disabled={saving}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={createStaff}
              loading={saving}
              disabled={
                createDraft.username.trim().length < 3 ||
                createDraft.displayName.trim().length < 1 ||
                createDraft.password.length < 10
              }
            >
              Create account
            </Button>
          </>
        }
      >
        <div className="space-y-3">
          <Input
            label="Username"
            value={createDraft.username}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => setCreateDraft({ ...createDraft, username: event.target.value })}
          />
          <Input
            label="Display name"
            value={createDraft.displayName}
            onChange={(event) => setCreateDraft({ ...createDraft, displayName: event.target.value })}
          />
          <Input
            label="Initial password"
            type="password"
            hint="minimum 10 characters"
            value={createDraft.password}
            autoComplete="new-password"
            onChange={(event) => setCreateDraft({ ...createDraft, password: event.target.value })}
          />
          <Select
            label="Role"
            value={createDraft.roleKey}
            onChange={(event) => setCreateDraft({ ...createDraft, roleKey: event.target.value })}
          >
            {options
              .filter((role) => role.key !== "owner")
              .map((role) => (
                <option key={role.key} value={role.key}>
                  {role.name}
                </option>
              ))}
          </Select>
          <Input
            label="Roblox user ID (optional)"
            hint="links the staff account to a Roblox player"
            value={createDraft.robloxUserId}
            inputMode="numeric"
            onChange={(event) => setCreateDraft({ ...createDraft, robloxUserId: event.target.value })}
          />
        </div>
      </Modal>

      <Modal
        open={resetTarget !== null}
        onClose={() => setResetTarget(null)}
        title={`Reset password — ${resetTarget?.displayName ?? ""}`}
        description="Existing sessions for this account stay valid until they expire or the account is disabled."
        footer={
          <>
            <Button variant="ghost" onClick={() => setResetTarget(null)} disabled={saving}>
              Cancel
            </Button>
            <Button
              variant="primary"
              loading={saving}
              disabled={resetPassword.length < 10}
              onClick={async () => {
                if (!token || !resetTarget) return;
                const ok = await run(
                  () => api.resetStaffPassword(token, resetTarget.id, resetPassword),
                  "Password reset",
                );
                if (ok) {
                  setResetTarget(null);
                  setResetPassword("");
                }
              }}
            >
              Set password
            </Button>
          </>
        }
      >
        <Input
          label="New password"
          type="password"
          hint="minimum 10 characters"
          value={resetPassword}
          autoComplete="new-password"
          onChange={(event) => setResetPassword(event.target.value)}
        />
      </Modal>

      <Modal
        open={roleOpen}
        onClose={() => setRoleOpen(false)}
        title={roleDraft.id === null ? "New role" : `Edit role — ${roleDraft.name}`}
        description="Select the permissions this role grants. Owner-only powers remain protected."
        width="max-w-2xl"
        footer={
          <>
            <Button variant="ghost" onClick={() => setRoleOpen(false)} disabled={saving}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={saveRole}
              loading={saving}
              disabled={roleDraft.name.trim().length < 2 || (roleDraft.id === null && !/^[a-z0-9_-]{2,32}$/.test(roleDraft.key))}
            >
              {roleDraft.id === null ? "Create role" : "Save role"}
            </Button>
          </>
        }
      >
        <div className="grid gap-3">
          <div className="grid gap-3 sm:grid-cols-2">
            {roleDraft.id === null ? (
              <Input
                label="Key"
                hint="lowercase, used by the backend"
                value={roleDraft.key}
                spellCheck={false}
                onChange={(event) => setRoleDraft({ ...roleDraft, key: event.target.value })}
              />
            ) : (
              <div>
                <span className="label">Key</span>
                <p className="mono text-[12px] text-muted">{roleDraft.key}</p>
              </div>
            )}
            <Input
              label="Name"
              value={roleDraft.name}
              onChange={(event) => setRoleDraft({ ...roleDraft, name: event.target.value })}
            />
          </div>
          <Textarea
            label="Description"
            rows={2}
            value={roleDraft.description}
            onChange={(event) => setRoleDraft({ ...roleDraft, description: event.target.value })}
          />
          <div className="max-h-[340px] space-y-3 overflow-y-auto rounded border border-line bg-surface-2/40 p-2.5">
            {groupedPermissions.map(([category, permissions]) => (
              <div key={category}>
                <p className="mb-1 text-[10px] font-medium uppercase tracking-[0.08em] text-faint">
                  {category}
                </p>
                <div className="grid gap-1 sm:grid-cols-2">
                  {permissions.map((permission) => (
                    <Checkbox
                      key={permission.key}
                      checked={roleDraft.permissions.includes(permission.key)}
                      onChange={(next) => toggleRolePermission(permission.key, next)}
                      label={
                        <span className="block">
                          <span className="block text-[11px] text-ink">{permission.label}</span>
                          <span className="mono block text-[9px] text-faint">{permission.key}</span>
                        </span>
                      }
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={deleteRole !== null}
        onClose={() => setDeleteRole(null)}
        title={`Delete role — ${deleteRole?.name ?? ""}`}
        description="This permanently removes the role. Only roles without assigned accounts can be deleted."
        confirmLabel="Delete role"
        destructive
        loading={saving}
        onConfirm={async () => {
          if (!token || !deleteRole) return;
          const ok = await run(() => api.deleteRole(token, deleteRole.id), "Role deleted");
          if (ok) setDeleteRole(null);
        }}
      />
    </div>
  );
}
