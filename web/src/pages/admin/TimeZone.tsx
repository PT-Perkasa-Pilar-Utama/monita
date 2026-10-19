import type { ReactElement } from "react";
import { TimeZoneSection } from "@/components/app/TimeZoneSection";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import type { SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Admin, Time zone: the UTC offset every page shows times in. */
export function TimeZone(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  return (
    <>
      <PageHeader title="Time zone" />
      {res.data ? <TimeZoneSection initial={res.data.utc_offset_min} /> : <Skeleton className="h-64 rounded-xl" />}
    </>
  );
}
