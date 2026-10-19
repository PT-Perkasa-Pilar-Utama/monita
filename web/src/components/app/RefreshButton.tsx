import type { ReactElement } from "react";
import { RefreshCw } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";

/** Reads the page again now. Live pages also read again on their own every 30 seconds. */
export function RefreshButton({ onClick, busy }: { onClick: () => void; busy: boolean }): ReactElement {
  return (
    <Button variant="outline" onClick={onClick} disabled={busy}>
      {busy ? <Spinner data-icon="inline-start" /> : <RefreshCw data-icon="inline-start" />}
      Refresh
    </Button>
  );
}
