import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { useSearchParams } from "react-router-dom";
import { CopyField } from "@/components/app/CopyField";
import { TableCard } from "@/components/app/TableCard";
import { PageHeader } from "@/components/layout/Page";
import { FilterMenu } from "@/components/list/FilterMenu";
import { PageSearch } from "@/components/list/PageSearch";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api, errorText } from "@/lib/api";
import { useMe } from "@/lib/session";
import { sortRows } from "@/lib/sort";
import type { AdminUserRow, Invited } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useSort } from "@/lib/useSort";
import { ACTION_LABEL, availableActions, planAction, type UserAction } from "@/lib/users";
import { toast } from "sonner";
import { ActionDialog } from "./ActionDialog";
import { Writable } from "@/lib/session";
import { InviteSheet } from "./InviteSheet";
import { UsersTable } from "./UsersTable";

const PAGE = 20;

const CALL: Record<UserAction, (id: number) => Promise<unknown>> = {
  make_admin: (id) => api(`/admin/users/${id}/role`, "PUT", { role: "super_admin" }),
  make_tl: (id) => api(`/admin/users/${id}/role`, "PUT", { role: "tl" }),
  make_supervisor: (id) => api(`/admin/users/${id}/role`, "PUT", { role: "supervisor" }),
  reset: (id) => api<Invited>(`/admin/users/${id}/invite`, "POST"),
  two_factor_off: (id) => api(`/admin/users/${id}/two-factor/off`, "POST"),
  disable: (id) => api(`/admin/users/${id}/disable`, "POST"),
  enable: (id) => api(`/admin/users/${id}/enable`, "POST"),
};

/** Admin, Users: every account, searched, filtered and sorted, with actions per row or on a selection. */
export function Users(): ReactElement {
  const me = useMe().email;
  const res = useApi<AdminUserRow[]>("/admin/users");
  const [params, setParams] = useSearchParams();
  const q = params.get("q") ?? "";
  const role = params.get("role") ?? "";
  const status = params.get("status") ?? "";
  const [sort, onSort] = useSort(["email", "role", "status", "added"]);
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [pending, setPending] = useState<{ action: UserAction; rows: AdminUserRow[] } | null>(null);
  const [invited, setInvited] = useState<Invited | null>(null);
  const [running, setRunning] = useState(false);

  const set = (patch: Record<string, string>): void =>
    setParams(
      (p) => {
        const out = new URLSearchParams(p);
        for (const [k, v] of Object.entries(patch)) v ? out.set(k, v) : out.delete(k);
        return out;
      },
      { replace: true },
    );
  useEffect(() => setPage(1), [q, role, status, sort?.by, sort?.dir]);

  const all = res.data ?? [];
  const needle = q.trim().toLowerCase();
  const shown = sortRows(
    all.filter((u) => (!needle || u.email.includes(needle)) && (!role || u.role === role) && (!status || u.status === status)),
    sort,
    { email: (u) => u.email, role: (u) => u.role, status: (u) => u.status, added: (u) => u.created_at },
  );
  const last = Math.max(1, Math.ceil(shown.length / PAGE));
  const pageRows = shown.slice((Math.min(page, last) - 1) * PAGE, Math.min(page, last) * PAGE);
  const chosen = all.filter((u) => selected.has(u.id));
  const filtered = needle !== "" || role !== "" || status !== "";

  const toggle = (id: number): void =>
    setSelected((s) => {
      const n = new Set(s);
      n.has(id) ? n.delete(id) : n.add(id);
      return n;
    });
  const togglePage = (on: boolean): void =>
    setSelected((s) => {
      const n = new Set(s);
      for (const u of pageRows) if (u.email !== me) on ? n.add(u.id) : n.delete(u.id);
      return n;
    });

  const run = async (action: UserAction, rows: AdminUserRow[]): Promise<void> => {
    const { targets } = planAction(action, rows, me);
    setPending(null);
    setRunning(true);
    const results = await Promise.allSettled(targets.map((u) => CALL[action](u.id)));
    setRunning(false);
    const failed = results.filter((r) => r.status === "rejected");
    const one = results[0];
    if (action === "reset" && targets.length === 1 && one?.status === "fulfilled") setInvited(one.value as Invited);
    if (failed.length === 0) toast.success(`${ACTION_LABEL[action]}: done for ${targets.length === 1 ? targets[0]!.email : `${targets.length} users`}.`);
    else toast.error(`${targets.length - failed.length} done, ${failed.length} failed: ${errorText((failed[0] as PromiseRejectedResult).reason)}`);
    setSelected(new Set());
    res.reload();
  };

  const plan = pending ? planAction(pending.action, pending.rows, me) : null;
  return (
    <>
      <PageHeader title="User management" filtered={filtered} actions={<Writable><InviteSheet onInvited={(i) => (setInvited(i), res.reload())} /></Writable>} />
      {invited && (
        <Alert role="status" className="mb-5">
          <AlertTitle>Invite sent to {invited.email} with a link that works once and expires in 72 hours.</AlertTitle>
          <AlertDescription className="mt-2">
            <CopyField id="invite-link" value={invited.link} />
          </AlertDescription>
        </Alert>
      )}
      <div className="flex flex-col gap-4">
        <TableCard
          onClear={filtered && chosen.length === 0 ? () => set({ q: "", role: "", status: "" }) : undefined}
          title={
            chosen.length > 0 ? (
              <span className="flex flex-wrap items-center gap-2">
                <span aria-live="polite">{chosen.length} selected</span>
                {availableActions(chosen, me).map((a) => (
                  <Button key={a} size="sm" variant={a === "disable" ? "danger" : "secondary"} disabled={running} onClick={() => setPending({ action: a, rows: chosen })}>
                    {ACTION_LABEL[a]}
                  </Button>
                ))}
                <Button size="sm" variant="ghost" disabled={running} onClick={() => setSelected(new Set())}>
                  Clear selection
                </Button>
              </span>
            ) : res.data ? (
              `${shown.length} ${shown.length === 1 ? "user" : "users"}${filtered ? ` of ${all.length}` : ""}`
            ) : (
              "Users"
            )
          }
          tools={
            <>
              <PageSearch value={q} onChange={(v) => set({ q: v })} placeholder="Search by email" />
              <FilterMenu
                label="Role"
                heading="Filter by role"
                value={role}
                options={[
                  { value: "", label: "Any role" },
                  { value: "super_admin", label: "Super admin" },
                  { value: "supervisor", label: "Supervisor" },
                  { value: "tl", label: "Tech lead" },
                ]}
                onChange={(v) => set({ role: v })}
              />
              <FilterMenu
                label="Status"
                heading="Filter by status"
                value={status}
                options={[
                  { value: "", label: "Any status" },
                  { value: "active", label: "Active" },
                  { value: "invited", label: "Invited" },
                  { value: "disabled", label: "Disabled" },
                ]}
                onChange={(v) => set({ status: v })}
              />
            </>
          }
        >
          {!res.data ? (
            <Skeleton className="m-5 h-40" />
          ) : shown.length === 0 ? (
            <TableEmpty title="No users here." filtered noun="users" onClear={() => set({ q: "", role: "", status: "" })} />
          ) : (
            <UsersTable rows={pageRows} me={me} selected={selected} onToggle={toggle} onTogglePage={togglePage} sort={sort} onSort={onSort} onAction={(a, u) => setPending({ action: a, rows: [u] })} />
          )}
        </TableCard>
        <Pager page={Math.min(page, last)} pageSize={PAGE} total={shown.length} noun="user" onPage={setPage} />
      </div>
      {pending && plan && <ActionDialog action={pending.action} targets={plan.targets} skipped={plan.skipped} onCancel={() => setPending(null)} onConfirm={() => void run(pending.action, pending.rows)} />}
    </>
  );
}
