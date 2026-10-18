import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { StageRing } from "@/components/app/TicketBoard";
import { Plus } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import { STAGES } from "@/lib/tickets";
import type { Stage } from "@/lib/types";
import { toast } from "sonner";

// Done needs a close reason, so a new ticket starts in one of the other three.
const OPEN_STAGES = STAGES.filter(([k]) => k !== "done");

/** The team files a ticket itself, with no reporter and no ref ID. */
export function NewTicketDialog({ slug, onFiled }: { slug: string; onFiled: (n: number) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [stage, setStage] = useState<Stage>("backlog");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const reset = (): void => {
    setTitle("");
    setDescription("");
    setStage("backlog");
    setError(null);
  };
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const { number } = await api<{ number: number }>(`/projects/${slug}/tickets`, "POST", { title, description, stage });
      toast.success(`Ticket #${number} filed.`);
      setOpen(false);
      reset();
      onFiled(number);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const err = (k: string): string | undefined => error?.fields[k];
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus data-icon="inline-start" />
          New ticket
        </Button>
      </DialogTrigger>
      <DialogContent aria-describedby={undefined} className="sm:max-w-2xl">
        <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>New ticket</DialogTitle>
          </DialogHeader>
          <FieldGroup className="gap-5">
            <Field data-invalid={err("title") ? true : undefined}>
              <FieldLabel htmlFor="ticket-title" required>
                Title
              </FieldLabel>
              <Input
                id="ticket-title"
                maxLength={120}
                placeholder="Refund button stays grey after payment"
                aria-required
                aria-invalid={err("title") ? true : undefined}
                value={title}
                onChange={(e) => setTitle(e.target.value)}
              />
              {err("title") && <FieldError>{err("title")}</FieldError>}
            </Field>
            <Field data-invalid={err("description") ? true : undefined}>
              <FieldLabel htmlFor="ticket-description" required>
                Description, only your team sees it
              </FieldLabel>
              <Textarea
                id="ticket-description"
                rows={8}
                placeholder="What happens, where, and how to make it happen again."
                aria-required
                aria-invalid={err("description") ? true : undefined}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
              {err("description") && <FieldError>{err("description")}</FieldError>}
            </Field>
            <FieldSet data-invalid={err("stage") ? true : undefined}>
              <FieldLegend variant="label">Column</FieldLegend>
              <ToggleGroup type="single" variant="outline" value={stage} onValueChange={(v) => v && setStage(v as Stage)} className="flex-wrap">
                {OPEN_STAGES.map(([k, l]) => (
                  <ToggleGroupItem key={k} value={k} className="px-3">
                    <StageRing stage={k} />
                    {l}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
              {err("stage") && <FieldError>{err("stage")}</FieldError>}
            </FieldSet>
            {error && Object.keys(error.fields).length === 0 && <p className="text-sm text-destructive">{error.message}</p>}
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              File ticket
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
