import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { IconPicker } from "@/components/app/IconPicker";
import { Activity, AlertTriangle, ExternalLink, Folder, Globe, Heartbeat, Key, Shield, Ticket, Users } from "@/components/icons";
import type { AppIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import type { ProjectHead } from "@/lib/types";
import { utc } from "@/lib/time";
import { toast } from "sonner";

const plural = (n: number, noun: string): string => `${n.toLocaleString()} ${noun}${n === 1 ? "" : "s"}`;

function Fact({ icon: Icon, children }: { icon: AppIcon; children: ReactNode }): ReactElement {
  return (
    <li className="flex items-start gap-2.5 text-sm">
      <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1 break-words">{children}</span>
    </li>
  );
}

/** The project's in-place edit: what the About column shows as a form, and its save. */
export type ProjectEdit = {
  editing: boolean;
  saving: boolean;
  description: string;
  setDescription: (v: string) => void;
  icon: string;
  setIcon: (v: string) => void;
  removeIcon: boolean;
  setRemoveIcon: (v: boolean) => void;
  error: ApiError | null;
  start: () => void;
  discard: () => void;
  save: () => Promise<void>;
};

export function useProjectEdit(head: ProjectHead, reload: () => void): ProjectEdit {
  const p = head.project;
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [description, setDescription] = useState("");
  const [icon, setIcon] = useState("");
  const [removeIcon, setRemoveIcon] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  return {
    editing,
    saving,
    description,
    setDescription,
    icon,
    setIcon,
    removeIcon,
    setRemoveIcon,
    error,
    start: () => {
      setDescription(p.description ?? "");
      setIcon("");
      setRemoveIcon(false);
      setError(null);
      setEditing(true);
    },
    discard: () => setEditing(false),
    save: async () => {
      setSaving(true);
      setError(null);
      try {
        await api(`/projects/${p.slug}/profile`, "PUT", { description, icon, remove_icon: removeIcon ? "on" : "" });
        reload();
        setEditing(false);
        toast.success("Project saved.");
      } catch (e) {
        setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
      } finally {
        setSaving(false);
      }
    },
  };
}

/** The About column: the project's facts, or its edit form while editing. */
export function ProjectAbout({
  head,
  activeKeys,
  triage,
  edit,
}: {
  head: ProjectHead;
  activeKeys: number;
  triage: number;
  edit: ProjectEdit;
}): ReactElement {
  const { project: p, counts } = head;
  const base = `/projects/${p.slug}`;
  return (
    <aside aria-labelledby="about-title" className="flex flex-col gap-4 lg:sticky lg:top-[calc(var(--bar-h)+24px)]">
      <h2 id="about-title" className="text-lg font-semibold">
        About
      </h2>
      {edit.editing ? (
        <FieldGroup className="gap-5">
          <Field data-invalid={edit.error?.fields.description ? true : undefined}>
            <FieldLabel htmlFor="about-description">Description</FieldLabel>
            <Textarea
              id="about-description"
              rows={4}
              maxLength={280}
              autoFocus
              placeholder="What this system does and who uses it."
              value={edit.description}
              onChange={(e) => edit.setDescription(e.target.value)}
            />
            {edit.error?.fields.description && <FieldError>{edit.error.fields.description}</FieldError>}
          </Field>
          <IconPicker value={edit.icon} onChange={edit.setIcon} current={edit.removeIcon ? null : p.icon_url} error={edit.error?.fields.icon} />
          {p.icon_url && !edit.icon && (
            <Field orientation="horizontal">
              <Checkbox id="about-remove-icon" checked={edit.removeIcon} onCheckedChange={(v) => edit.setRemoveIcon(v === true)} />
              <FieldLabel htmlFor="about-remove-icon" className="font-normal">
                Remove the icon and show the first letter
              </FieldLabel>
            </Field>
          )}
          {edit.error && !edit.error.fields.description && !edit.error.fields.icon && (
            <p className="text-sm text-destructive">{edit.error.message}</p>
          )}
        </FieldGroup>
      ) : p.description ? (
        <p className="text-sm text-foreground/90">{p.description}</p>
      ) : (
        <p className="text-sm text-muted-foreground">No description.</p>
      )}
      <ul className="flex flex-col gap-2.5">
        <Fact icon={Globe}>
          <span className="flex items-center justify-between gap-2">
            Status page
            <Button asChild variant="outline" size="sm" className="-my-1.5 h-7">
              <a href={`/status/${p.slug}`} target="_blank" rel="noreferrer" title="Open the status page in a new tab">
                Open
                <ExternalLink data-icon="inline-end" />
              </a>
            </Button>
          </span>
        </Fact>
        <Fact icon={Users}>Owned by {p.owner_email}</Fact>
        <Fact icon={Folder}>Created {utc(p.created_at)}</Fact>
        <Fact icon={Key}>
          <Link to={`${base}/settings/keys`} className="text-link hover:underline">
            {plural(activeKeys, "active API key")}
          </Link>
        </Fact>
        <Fact icon={Shield}>
          <Link to={`${base}/settings/allowlist`} className="hover:underline">
            {p.ip_allowlist ? "IP allowlist on" : "IP allowlist off"}
          </Link>
        </Fact>
      </ul>
      <Separator />
      <ul className="flex flex-col gap-2.5">
        <Fact icon={Activity}>{plural(counts.monitors, "monitor")}</Fact>
        <Fact icon={Heartbeat}>{plural(counts.heartbeats, "heartbeat")}</Fact>
        <Fact icon={AlertTriangle}>{plural(counts.incidents, "open incident")}</Fact>
        <Fact icon={Ticket}>
          {plural(counts.tickets, "open ticket")}
          <span className="block text-xs text-muted-foreground">{triage} in Triage</span>
        </Fact>
      </ul>
    </aside>
  );
}
