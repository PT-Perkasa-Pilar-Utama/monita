import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { IconPicker } from "@/components/app/IconPicker";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { UserPicker } from "@/components/app/UserPicker";
import { Button } from "@/components/ui/button";
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
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";
import { stamp } from "@/lib/time";
import type { Owner } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const asApiError = (e: unknown): ApiError => (e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));

/** Name, slug, homepage, description and icon. The name and the slug never change. */
function ProjectSection(): ReactElement {
  const { project: p, reload } = useProject();
  const [description, setDescription] = useState(p.description ?? "");
  const [homepage, setHomepage] = useState(p.homepage_url ?? "");
  const [icon, setIcon] = useState("");
  const [removeIcon, setRemoveIcon] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const changed = description !== (p.description ?? "") || homepage !== (p.homepage_url ?? "") || icon !== "" || removeIcon;
  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await api(`/projects/${p.slug}/profile`, "PUT", { description, homepage_url: homepage, icon, remove_icon: removeIcon ? "on" : "" });
      reload();
      setIcon("");
      setRemoveIcon(false);
      toast.success("Project saved.");
    } catch (err) {
      setError(asApiError(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <SettingsSection
      id="project"
      title="Project"
      description="The name and the slug stay as they are."
      onSubmit={() => void save()}
      footerNote={`Created ${stamp(p.created_at)}`}
      footer={
        <Button type="submit" disabled={busy || !changed}>
          {busy && <Spinner data-icon="inline-start" />}
          Save changes
        </Button>
      }
    >
      <FieldGroup className="gap-5">
        <div className="grid gap-5 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor="general-name">Name</FieldLabel>
            <Input id="general-name" value={p.name} readOnly disabled />
          </Field>
          <Field>
            <FieldLabel htmlFor="general-slug">Slug</FieldLabel>
            <Input id="general-slug" value={p.slug} readOnly disabled className="font-mono" />
          </Field>
        </div>
        <Field data-invalid={error?.fields.homepage_url ? true : undefined}>
          <FieldLabel htmlFor="general-homepage" required>
            Homepage
          </FieldLabel>
          <Input
            id="general-homepage"
            type="url"
            className="font-mono"
            placeholder="https://payments.example.com"
            aria-required
            aria-invalid={error?.fields.homepage_url ? true : undefined}
            value={homepage}
            onChange={(e) => setHomepage(e.target.value)}
          />
          {error?.fields.homepage_url && <FieldError>{error.fields.homepage_url}</FieldError>}
        </Field>
        <Field data-invalid={error?.fields.description ? true : undefined}>
          <FieldLabel htmlFor="general-description">Description</FieldLabel>
          <Textarea
            id="general-description"
            rows={3}
            maxLength={280}
            placeholder="What this system does and who uses it."
            aria-invalid={error?.fields.description ? true : undefined}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
          {error?.fields.description && <FieldError>{error.fields.description}</FieldError>}
        </Field>
        <IconPicker value={icon} onChange={setIcon} current={removeIcon ? null : p.icon_url} error={error?.fields.icon} />
        {p.icon_url && !icon && (
          <Field orientation="horizontal">
            <Checkbox id="general-remove-icon" checked={removeIcon} onCheckedChange={(v) => setRemoveIcon(v === true)} />
            <FieldLabel htmlFor="general-remove-icon" className="font-normal">
              Remove the icon and show the first letter
            </FieldLabel>
          </Field>
        )}
        {error && !error.fields.description && !error.fields.homepage_url && !error.fields.icon && <p className="text-sm text-destructive">{error.message}</p>}
      </FieldGroup>
    </SettingsSection>
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

/** Typing the name again guards the one action that cannot be undone. */
function DeleteProject({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }): ReactElement {
  const { project: p } = useProject();
  const navigate = useNavigate();
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await api(`/projects/${p.slug}`, "DELETE", { confirm });
      toast.success(`${p.name} is deleted.`);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.confirm ?? err.message) : errorText(err));
      setBusy(false);
    }
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Delete {p.name}?</DialogTitle>
            <DialogDescription>
              This deletes its monitors, heartbeats, incidents, tickets, API keys and status page. It cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <Field data-invalid={error ? true : undefined}>
            <FieldLabel htmlFor="delete-confirm">Type {p.name} to confirm</FieldLabel>
            <Input id="delete-confirm" autoComplete="off" value={confirm} onChange={(e) => setConfirm(e.target.value)} aria-invalid={error ? true : undefined} />
            {error && <FieldError>{error}</FieldError>}
          </Field>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" variant="destructive" disabled={busy || confirm.trim() !== p.name}>
              {busy && <Spinner data-icon="inline-start" />}
              Delete this project
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** Settings, General: the project, its status page, its owner and, for a super admin, deletion. */
export function General(): ReactElement {
  const me = useMe();
  const { project: p } = useProject();
  const [reassigning, setReassigning] = useState(false);
  const [deleting, setDeleting] = useState(false);
  return (
    <SettingsPage
      title="General"
      toc={[
        { id: "project", label: "Project" },
        { id: "owner", label: "Owner" },
        ...(me.is_admin ? [{ id: "danger-zone", label: "Danger zone" }] : []),
      ]}
    >
      <ProjectSection />
      <SettingsSection
        id="owner"
        title="Owner"
        description="The owner manages this project. Its monitors, keys and tickets stay when it moves to another tech lead."
        footer={
          me.is_admin && (
            <Button variant="outline" onClick={() => setReassigning(true)}>
              Reassign
            </Button>
          )
        }
      >
        <p className="text-sm font-medium">{p.owner_email}</p>
      </SettingsSection>
      {me.is_admin && (
        <SettingsSection id="danger-zone" title="Danger zone" description="Only a super admin sees this section." danger>
          <div className="grid items-center gap-4 sm:grid-cols-[minmax(0,1fr)_auto]">
            <div className="flex min-w-0 flex-col gap-1">
              <strong className="text-sm font-semibold">Delete this project</strong>
              <span className="text-sm text-muted-foreground">Its monitors, heartbeats, incidents, tickets and keys go with it. This cannot be undone.</span>
            </div>
            <Button variant="danger" onClick={() => setDeleting(true)}>
              Delete this project
            </Button>
          </div>
        </SettingsSection>
      )}
      {me.is_admin && reassigning && <Reassign open={reassigning} onOpenChange={setReassigning} />}
      {me.is_admin && deleting && <DeleteProject open={deleting} onOpenChange={setDeleting} />}
    </SettingsPage>
  );
}
