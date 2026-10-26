import { UserAvatar } from "@/components/app/UserAvatar";
import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { IconPicker } from "@/components/app/IconPicker";
import { Activity, AlertTriangle, Bell, ExternalLink, Folder, Globe, Heartbeat, Key, Shield, Ticket } from "@/components/icons";
import type { AppIcon } from "@/components/icons";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { LEVELS, levelLine } from "@/lib/alertCards";
import { allowlistSize, publicTabs } from "@/lib/projectFacts";
import type { AlertCard, ProjectHead } from "@/lib/types";
import { stamp } from "@/lib/time";
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
  homepage: string;
  setHomepage: (v: string) => void;
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
  const [homepage, setHomepage] = useState("");
  const [icon, setIcon] = useState("");
  const [removeIcon, setRemoveIcon] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const clear = (field: string): void => setError((e) => e && dropField(e, field));
  return {
    editing,
    saving,
    description,
    setDescription: (v) => {
      setDescription(v);
      clear("description");
    },
    homepage,
    setHomepage: (v) => {
      setHomepage(v);
      clear("homepage_url");
    },
    icon,
    setIcon: (v) => {
      setIcon(v);
      clear("icon");
    },
    removeIcon,
    setRemoveIcon,
    error,
    start: () => {
      setDescription(p.description ?? "");
      setHomepage(p.homepage_url ?? "");
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
        await api(`/projects/${p.slug}/profile`, "PUT", { description, homepage_url: homepage, icon, remove_icon: removeIcon ? "on" : "" });
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
  alerts,
  edit,
}: {
  head: ProjectHead;
  activeKeys: number;
  triage: number;
  /** The project's alert cards; undefined while they load. */
  alerts: AlertCard[] | undefined;
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
          <Field data-invalid={edit.error?.fields.homepage_url ? true : undefined}>
            <FieldLabel htmlFor="about-homepage" required>
              Homepage
            </FieldLabel>
            <Input
              id="about-homepage"
              type="url"
              aria-required
              aria-invalid={edit.error?.fields.homepage_url ? true : undefined}
              className="font-mono"
              placeholder="https://payments.example.com"
              value={edit.homepage}
              onChange={(e) => edit.setHomepage(e.target.value)}
            />
            {edit.error?.fields.homepage_url && <FieldError>{edit.error.fields.homepage_url}</FieldError>}
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
          {edit.error && Object.keys(edit.error.fields).length === 0 && (
            <p className="text-sm text-destructive">{edit.error.message}</p>
          )}
        </FieldGroup>
      ) : p.description ? (
        <p className="text-sm text-foreground/90">{p.description}</p>
      ) : (
        <p className="text-sm text-muted-foreground">No description.</p>
      )}
      <ul className="flex flex-col gap-2.5">
        {p.homepage_url && (
          <Fact icon={ExternalLink}>
            <a href={p.homepage_url} target="_blank" rel="noreferrer" className="break-all text-link hover:underline">
              {p.homepage_url.replace(/^https?:\/\//, "")}
            </a>
          </Fact>
        )}
        <li className="flex items-center gap-2.5 text-sm">
          <UserAvatar id={p.owner_id} email={p.owner_email} avatarAt={p.owner_avatar_at} className="size-6 text-[11px]" />
          <span className="min-w-0 break-all">Owned by {p.owner_email}</span>
        </li>
        <Fact icon={Folder}>Created {stamp(p.created_at)}</Fact>
        <Fact icon={Key}>
          <Link to={`${base}/settings/keys`} className="text-link hover:underline">
            {plural(activeKeys, "active API key")}
          </Link>
        </Fact>
      </ul>
      <Separator />
      <ul className="flex flex-col gap-3">
        <Fact icon={Globe}>
          <Link to={`${base}/settings/status-page`} className="text-link hover:underline">
            Status page tabs
          </Link>
          <span className="block text-muted-foreground">{publicTabs(p.pages).join(", ")}</span>
        </Fact>
        <Fact icon={Bell}>
          <Link to={`${base}/settings/alerts`} className="text-link hover:underline">
            Alerts
          </Link>
          {alerts ? (
            LEVELS.map((l) => (
              <span key={l} className="block text-muted-foreground">
                {levelLine(alerts, l)}
              </span>
            ))
          ) : (
            <span className="block text-muted-foreground">Loading</span>
          )}
        </Fact>
        <Fact icon={Shield}>
          <Link to={`${base}/settings/allowlist`} className="text-link hover:underline">
            IP allowlist
          </Link>
          <span className="block text-muted-foreground">
            {allowlistSize(p.ip_allowlist) === 0
              ? "Off, so every address can call the API."
              : `On, so only ${allowlistSize(p.ip_allowlist) === 1 ? "1 address or range" : `${allowlistSize(p.ip_allowlist)} addresses or ranges`} may call the API.`}
          </span>
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
