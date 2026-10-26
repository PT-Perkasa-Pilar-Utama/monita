import { useId, useState } from "react";
import { Writable } from "@/lib/session";
import type { ReactElement } from "react";
import { Upload, X } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { shrinkIcon } from "@/lib/icon";

/**
 * A small picture field: a project icon, or a user's own picture. The picture is cropped and
 * scaled in the page, then sent as a data URL in `value`; an empty value sends none. `current` is
 * the stored picture, shown until replaced. `own` keeps the button for a supervisor, who may change
 * their own account.
 */
export function IconPicker({
  value,
  onChange,
  current,
  error,
  label = "Icon",
  own = false,
}: {
  value: string;
  onChange: (dataUrl: string) => void;
  current?: string | null | undefined;
  error?: string | undefined;
  label?: string;
  own?: boolean;
}): ReactElement {
  const id = useId();
  const [readError, setReadError] = useState("");
  const shown = value || current || "";
  const pick = async (file: File | undefined): Promise<void> => {
    setReadError("");
    if (!file) return;
    try {
      onChange(await shrinkIcon(file));
    } catch {
      setReadError("Monita could not read this picture, so pick a PNG, JPEG or WebP file.");
    }
  };
  const message = readError || error;
  const choose = (
    <Button asChild variant="outline" size="sm">
      <label htmlFor={id} className="cursor-pointer">
        {shown ? "Replace picture" : "Choose a picture"}
      </label>
    </Button>
  );
  return (
    <Field data-invalid={message ? true : undefined}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <div className="flex items-center gap-3">
        <span className="grid size-12 shrink-0 place-items-center overflow-hidden rounded-lg border bg-muted">
          {shown ? <img src={shown} alt="" data-icon-preview className="size-full object-cover" /> : <Upload className="size-5 text-muted-foreground" />}
        </span>
        {own ? choose : <Writable>{choose}</Writable>}
        <input
          id={id}
          type="file"
          accept="image/png,image/jpeg,image/webp"
          className="sr-only"
          aria-invalid={message ? true : undefined}
          onChange={(e) => void pick(e.target.files?.[0])}
        />
        {value && (
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Clear the new picture" onClick={() => onChange("")}>
            <X />
          </Button>
        )}
      </div>
      {message && <FieldError>{message}</FieldError>}
    </Field>
  );
}
