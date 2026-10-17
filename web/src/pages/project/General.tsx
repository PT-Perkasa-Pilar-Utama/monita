import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Facts } from "@/components/app/Facts";
import { IconPicker } from "@/components/app/IconPicker";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { UserPicker } from "@/components/app/UserPicker";
import { PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";
import { utc } from "@/lib/time";
import type { Owner } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const asApiError = (e: unknown): ApiError => (e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));

function EditProject({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }): ReactElement {
  const { project: p, reload } = useProject();
  const [description, setDescription] = useState(p.description ?? "");
  const [icon, setIcon] = useState("");
  const [removeIcon, setRemoveIcon] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`/projects/${p.slug}/profile`, "PUT", { description, icon, remove_icon: removeIcon ? "on" : "" });
      reload();
      onOpenChange(false);
      toast.success("Project saved.");
    } catch (err) {
      setError(asApiError(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Edit project</DialogTitle>
            <DialogDescription>The name and the slug stay as they are.</DialogDescription>
          </DialogHeader>
          <FieldGroup className="gap-5">
            <Field data-invalid={error?.fields.description ? true : undefined}>
              <FieldLabel htmlFor="edit-description">Description</FieldLabel>
              <Textarea id="edit-description" rows={3} maxLength={280} value={description} onChange={(e) => setDescription(e.target.value)} />
              {error?.fields.description && <FieldError>{error.fields.description}</FieldError>}
            </Field>
            <IconPicker value={icon} onChange={setIcon} current={removeIcon ? null : p.icon_url} error={error?.fields.icon} />
            {p.icon_url && !icon && (
              <Field orientation="horizontal">
                <Checkbox id="remove-icon" checked={removeIcon} onCheckedChange={(v) => setRemoveIcon(v === true)} />
                <FieldLabel htmlFor="remove-icon" className="font-normal">
                  Remove the icon and show the first letter
                </FieldLabel>
              </Field>
            )}
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Save project
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Reassign({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }): ReactElement {
  const { project: p, reload } = useProject();
  const owners = useApi<Owner[]>(open ? `/projects/${p.slug}/owners` : null);
  const [owner, setOwner] = useState(p.owner_email);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api(`/projects/${p.slug}/owner`, "PUT", { owner_email: owner });
      reload();
      onOpenChange(false);
      toast.success(`The project now belongs to ${owner}.`);
    } catch (err) {
      setError(asApiError(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Reassign this project</DialogTitle>
            <DialogDescription>The new owner gets the project with its monitors, keys and tickets. The old owner loses access to it.</DialogDescription>
          </DialogHeader>
          <UserPicker label="New owner" required users={owners.data ?? []} value={owner} onChange={setOwner} error={error?.fields.owner_email} />
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Reassign project
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** Settings, General: the project's facts, its owner, and the dialogs that change them. */
export function General(): ReactElement {
  const me = useMe();
  const { project: p } = useProject();
  const [editing, setEditing] = useState(false);
  const [reassigning, setReassigning] = useState(false);
  return (
    <>
      <PageHeader title="General" />
      <div className="grid items-start gap-6 xl:grid-cols-2">
        <Card>
          <CardHeader className="border-b">
            <CardTitle>Project</CardTitle>
            <CardAction>
              <Button variant="secondary" size="sm" onClick={() => setEditing(true)}>
                Edit
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent>
            <Facts
              rows={[
                ["Icon", <ProjectBadge key="icon" name={p.name} iconUrl={p.icon_url} className="size-9 rounded-lg text-sm" />],
                ["Name", p.name],
                ["Description", p.description ?? <span className="text-muted-foreground">None yet</span>],
                ["Slug", <span key="slug" className="font-mono">{p.slug}</span>],
                [
                  "Status page",
                  <a key="status" href={`/status/${p.slug}`} className="font-mono text-link hover:underline">
                    /status/{p.slug}
                  </a>,
                ],
                ["Created", utc(p.created_at)],
              ]}
            />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="border-b">
            <CardTitle>Owner</CardTitle>
            {me.is_admin && (
              <CardAction>
                <Button variant="secondary" size="sm" onClick={() => setReassigning(true)}>
                  Reassign
                </Button>
              </CardAction>
            )}
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <p className="font-medium">{p.owner_email}</p>
            <CardDescription>
              The owner manages this project. A super admin can give it to another tech lead; its monitors, keys and tickets stay as they are.
            </CardDescription>
          </CardContent>
        </Card>
      </div>
      {editing && <EditProject open={editing} onOpenChange={setEditing} />}
      {me.is_admin && reassigning && <Reassign open={reassigning} onOpenChange={setReassigning} />}
    </>
  );
}
