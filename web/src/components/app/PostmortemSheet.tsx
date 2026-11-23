import { useState } from "react";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { fillEmpty, publishNote, SECTIONS, template, UNFILLED, unfilled } from "@/lib/postmortem";
import type { IncidentDetail, Postmortem, PostmortemText } from "@/lib/types";

/** The post-mortem form (v1.2 Q63): Google's eight sections. Save keeps a draft; Publish asks
 * first and says who gets the email. */
export function PostmortemSheet({ path, pm, incident, open, onClose, onSaved }: { path: string; pm: Postmortem; incident: IncidentDetail; open: boolean; onClose: () => void; onSaved: () => void }): ReactElement {
  const [text, setText] = useState<PostmortemText>(() => Object.fromEntries(SECTIONS.map((s) => [s.key, pm[s.key]])) as PostmortemText);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const anyEmpty = SECTIONS.some((s) => text[s.key].trim() === "");

  // A prompt left from the template stops the publish before the dialog asks.
  const publish = (): void => {
    const left = unfilled(text);
    if (left.length === 0) return setConfirming(true);
    setError(new ApiError(422, "invalid", UNFILLED, Object.fromEntries(left.map((k) => [k, UNFILLED]))));
  };

  const send = async (action: "save" | "publish"): Promise<boolean> => {
    setBusy(true);
    try {
      const out = await api<{ state: string; emailed: number }>(`${path}/postmortem`, "PUT", { action, ...text });
      setError(null);
      onSaved();
      onClose();
      toast.success(action === "save" ? "Draft saved." : out.emailed > 0 ? `Post-mortem published and emailed to ${out.emailed}.` : "Post-mortem published.");
      return true;
    } catch (e) {
      if (e instanceof ApiError && Object.keys(e.fields).length > 0) setError(e);
      else toast.error(errorText(e));
      return false;
    } finally {
      setBusy(false);
    }
  };

  return (
    <FormSheet open={open} onClose={onClose}>
      <FormSheetBody
        title={pm.state === "none" ? "Write the post-mortem" : "Edit the post-mortem"}
        description="Summary, root cause and resolution are needed to publish, and a draft can be saved at any time."
        onSubmit={() => void send("save")}
        footer={
          <>
            {pm.state === "draft" && pm.published_at === null && (
              <ConfirmDialog
                trigger={
                  <Button type="button" variant="ghost" className="text-destructive">
                    Delete draft
                  </Button>
                }
                title="Delete this draft?"
                description="Its text is gone for good."
                action="Delete draft"
                onConfirm={async () => {
                  await api(`${path}/postmortem`, "DELETE");
                  onSaved();
                  onClose();
                }}
              />
            )}
            <span className="flex-1" />
            <Button type="submit" variant="outline" disabled={busy}>
              {busy && !confirming && <Spinner data-icon="inline-start" />}
              Save draft
            </Button>
            <ConfirmDialog
              open={confirming}
              onOpenChange={setConfirming}
              title="Publish the post-mortem?"
              description={publishNote(pm.recipients)}
              action="Publish"
              onConfirm={async () => {
                await send("publish");
              }}
            />
            <Button type="button" disabled={busy} onClick={publish}>
              Publish
            </Button>
          </>
        }
      >
        <FieldGroup className="gap-5">
          <Button type="button" variant="outline" size="sm" className="self-start" disabled={!anyEmpty} onClick={() => setText(fillEmpty(text, template(incident)))}>
            Fill in a template
          </Button>
          {SECTIONS.map((s) => {
            const message = error?.fields[s.key];
            return (
              <Field key={s.key} data-invalid={message ? true : undefined}>
                <FieldLabel htmlFor={`pm-${s.key}`} required={s.required}>
                  {s.label}
                </FieldLabel>
                <Textarea
                  id={`pm-${s.key}`}
                  rows={3}
                  maxLength={2000}
                  aria-invalid={message ? true : undefined}
                  value={text[s.key]}
                  onChange={(e) => {
                    setText({ ...text, [s.key]: e.target.value });
                    if (error) setError(dropField(error, s.key));
                  }}
                />
                {message && <FieldError>{message}</FieldError>}
              </Field>
            );
          })}
        </FieldGroup>
      </FormSheetBody>
    </FormSheet>
  );
}
