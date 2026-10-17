import { useState } from "react";
import type { ReactElement } from "react";
import { Check, Copy } from "@/components/icons";
import { Button } from "@/components/ui/button";

/** A value to copy, such as a new API key or a ping URL. `id` lets a test read the value. */
export function CopyField({ id, value }: { id: string; value: string }): ReactElement {
  const [copied, setCopied] = useState(false);
  const copy = async (): Promise<void> => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      window.getSelection()?.selectAllChildren(document.getElementById(id) ?? document.body);
    }
  };
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-lg border bg-paper p-1.5 pl-3">
      <code id={id} className="min-w-0 flex-1 truncate font-mono text-[13px]">
        {value}
      </code>
      <Button type="button" variant="secondary" size="sm" onClick={() => void copy()}>
        {copied ? <Check data-icon="inline-start" /> : <Copy data-icon="inline-start" />}
        {copied ? "Copied" : "Copy"}
      </Button>
    </div>
  );
}
