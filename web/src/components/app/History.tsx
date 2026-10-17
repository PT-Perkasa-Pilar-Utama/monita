import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { ArrowLeft, ArrowRight } from "@/components/icons";
import { Button } from "@/components/ui/button";

/** Links under a history table: back to the newest rows, and on to older ones. */
export function HistoryPager({ noun, older, newest }: { noun: string; older: string | null; newest: string | null }): ReactElement | null {
  if (!older && !newest) return null;
  return (
    <div className="flex items-center justify-between gap-3 border-t px-5 py-2">
      {newest ? (
        <Button asChild variant="ghost" size="sm">
          <Link to={newest} preventScrollReset>
            <ArrowLeft data-icon="inline-start" />
            Newest {noun}
          </Link>
        </Button>
      ) : (
        <span />
      )}
      {older && (
        <Button asChild variant="ghost" size="sm">
          <Link to={older} preventScrollReset>
            Older {noun}
            <ArrowRight data-icon="inline-end" />
          </Link>
        </Button>
      )}
    </div>
  );
}
