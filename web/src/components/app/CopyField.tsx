import { useState } from "react";
import type { ReactElement } from "react";
import { Check, Copy } from "@/components/icons";
import { Button } from "@/components/ui/button";

/** Copies `value`; if the clipboard is blocked, selects the element `targetId` so the user can. */
export function CopyButton({ value, targetId, label = "Copy" }: { value: string; targetId: string; label?: string }): ReactElement {
  const [copied, setCopied] = useState(false);
  const copy = async (): Promise<void> => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      window.getSelection()?.selectAllChildren(document.getElementById(targetId) ?? document.body);
    }
  };
  return (
    <Button type="button" variant="secondary" size="sm" onClick={() => void copy()} aria-label={`${label}: ${value.split("\n")[0]}`}>
      {copied ? <Check data-icon="inline-start" /> : <Copy data-icon="inline-start" />}
      {copied ? "Copied" : label}
    </Button>
  );
}

/** A value to copy, such as a new API key or a ping URL. `id` lets a test read the value. */
export function CopyField({ id, value }: { id: string; value: string }): ReactElement {
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-lg border bg-paper p-1.5 pl-3">
      <code id={id} className="min-w-0 flex-1 truncate font-mono text-[13px]">
        {value}
      </code>
      <CopyButton value={value} targetId={id} />
    </div>
  );
}

/** A command or script to copy, under its title. It scrolls sideways rather than wrap. */
export function CodeCopy({ id, title, code }: { id: string; title: string; code: string }): ReactElement {
  return (
    <div className="overflow-hidden rounded-lg border bg-paper">
      <div className="flex items-center justify-between gap-3 border-b bg-card py-1.5 pr-1.5 pl-3">
        <span className="text-sm font-medium">{title}</span>
        <CopyButton value={code} targetId={id} />
      </div>
      <pre className="overflow-x-auto p-3 font-mono text-[13px] leading-relaxed">
        <code id={id}>{code}</code>
      </pre>
    </div>
  );
}
