import type { ReactElement } from "react";
import { StateBadge } from "@/components/app/Tone";
import { MoreHorizontal, Shield } from "@/components/icons";
import { SortHead } from "@/components/list/SortHead";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { Sort } from "@/lib/sort";
import { day } from "@/lib/time";
import type { AdminUserRow } from "@/lib/types";
import { ACTION_LABEL, appliesTo, type UserAction } from "@/lib/users";

const STATUS = {
  active: ["up", "Active"],
  invited: ["issue", "Invited"],
  disabled: ["unknown", "Disabled"],
} as const;

function RoleBadge({ role }: { role: AdminUserRow["role"] }): ReactElement {
  return role === "super_admin" ? (
    <Badge variant="neutral">
      <Shield />
      Super admin
    </Badge>
  ) : (
    <Badge variant="outline">Tech lead</Badge>
  );
}

/** One row's actions: the role change, a new invite, then disable or enable. */
function RowMenu({ u, me, onAction }: { u: AdminUserRow; me: string; onAction: (a: UserAction, u: AdminUserRow) => void }): ReactElement | null {
  const fits = (a: UserAction): boolean => appliesTo(a, u, me);
  if (!(["make_admin", "make_tl", "reset", "disable", "enable"] as const).some(fits)) return null;
  const item = (a: UserAction): ReactElement | null =>
    fits(a) ? (
      <DropdownMenuItem key={a} variant={a === "disable" ? "destructive" : "default"} onSelect={() => onAction(a, u)}>
        {ACTION_LABEL[a]}
      </DropdownMenuItem>
    ) : null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="size-8" aria-label={`Actions for ${u.email}`}>
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuGroup>
          {item("make_admin")}
          {item("make_tl")}
          {item("reset")}
        </DropdownMenuGroup>
        {(fits("disable") || fits("enable")) && <DropdownMenuSeparator />}
        <DropdownMenuGroup>
          {item("enable")}
          {item("disable")}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** A page of accounts with a box per row for batch actions. Your own row has no box and no menu. */
export function UsersTable({
  rows,
  me,
  selected,
  onToggle,
  onTogglePage,
  sort,
  onSort,
  onAction,
}: {
  rows: AdminUserRow[];
  me: string;
  selected: ReadonlySet<number>;
  onToggle: (id: number) => void;
  onTogglePage: (on: boolean) => void;
  sort: Sort;
  onSort: (by: string) => void;
  onAction: (a: UserAction, u: AdminUserRow) => void;
}): ReactElement {
  const pickable = rows.filter((u) => u.email !== me);
  const picked = pickable.filter((u) => selected.has(u.id)).length;
  const all = picked === 0 ? false : picked === pickable.length ? true : "indeterminate";
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-10">
            <Checkbox aria-label="Select every user on this page" checked={all} disabled={pickable.length === 0} onCheckedChange={(on) => onTogglePage(on === true)} />
          </TableHead>
          <SortHead by="email" label="User" sort={sort} onSort={onSort} />
          <SortHead by="role" label="Role" sort={sort} onSort={onSort} />
          <SortHead by="status" label="Status" sort={sort} onSort={onSort} />
          <SortHead by="added" label="Added" sort={sort} onSort={onSort} className="max-md:hidden" />
          <TableHead className="w-12">
            <span className="sr-only">Actions</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((u) => {
          const self = u.email === me;
          return (
            <TableRow key={u.id} data-state={selected.has(u.id) ? "selected" : undefined}>
              <TableCell>{!self && <Checkbox aria-label={`Select ${u.email}`} checked={selected.has(u.id)} onCheckedChange={() => onToggle(u.id)} />}</TableCell>
              <TableCell className="max-w-72">
                <div className="flex items-center gap-3">
                  <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-full bg-muted text-[13px] font-semibold text-muted-foreground">
                    {u.email[0]?.toUpperCase()}
                  </span>
                  <span className="truncate font-medium">{u.email}</span>
                  {self && <Badge variant="outline">You</Badge>}
                </div>
              </TableCell>
              <TableCell>
                <RoleBadge role={u.role} />
              </TableCell>
              <TableCell>
                <StateBadge kind={STATUS[u.status][0]} label={STATUS[u.status][1]} />
              </TableCell>
              <TableCell className="text-muted-foreground tabular-nums max-md:hidden">{day(u.created_at)}</TableCell>
              <TableCell className="text-right">
                <RowMenu u={u} me={me} onAction={onAction} />
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
