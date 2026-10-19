import type { ReactElement } from "react";
import { Plus, X } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { FieldError, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { CHANGE_BOXES } from "@/lib/changelog";
import type { ChangeKey, ReleaseForm } from "@/lib/changelog";

/** The release's changes: per kind, one input per change and an Add change button. One input
 * is always one change, however long it runs. */
export function ReleaseChanges({ form, onChange, err }: { form: ReleaseForm; onChange: (k: ChangeKey, items: string[]) => void; err: (k: string) => string | undefined }): ReactElement {
  return (
    <FieldSet className="gap-6">
      <FieldLegend>Changes</FieldLegend>
      {CHANGE_BOXES.map(([tag, key]) => {
        const items = form[key];
        return (
          <FieldSet key={key} className="gap-2" data-invalid={err(key) ? true : undefined}>
            <FieldLegend variant="label">{tag}</FieldLegend>
            {items.map((item, i) => (
              <div key={i} className="flex gap-2">
                <Input
                  aria-label={`${tag} change ${i + 1}`}
                  autoFocus={item === "" && i === items.length - 1 && i > 0}
                  aria-invalid={err(key) ? true : undefined}
                  value={item}
                  onChange={(e) => onChange(key, items.map((x, j) => (j === i ? e.target.value : x)))}
                  onKeyDown={(e) => {
                    // Enter adds the next change instead of sending the form.
                    if (e.key === "Enter") {
                      e.preventDefault();
                      onChange(key, [...items.slice(0, i + 1), "", ...items.slice(i + 1)]);
                    }
                  }}
                  placeholder={i === 0 ? "What changed, in one sentence" : undefined}
                />
                <Button type="button" variant="ghost" size="icon" aria-label={`Remove ${tag} change ${i + 1}`} onClick={() => onChange(key, items.filter((_, j) => j !== i))}>
                  <X />
                </Button>
              </div>
            ))}
            {err(key) && <FieldError>{err(key)}</FieldError>}
            <div>
              <Button type="button" variant="outline" size="sm" aria-label={`Add a change to ${tag}`} onClick={() => onChange(key, [...items, ""])}>
                <Plus data-icon="inline-start" />
                Add a change
              </Button>
            </div>
          </FieldSet>
        );
      })}
    </FieldSet>
  );
}
