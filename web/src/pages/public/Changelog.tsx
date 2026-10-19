import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { useOutletContext, useParams } from "react-router-dom";
import { Neighbour, ReleaseCard, VersionIndex } from "@/components/app/ReleaseCard";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import type { PublicRelease, PublicReleaseIndex, PublicSite, ReleaseHead } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The public Changelog: one release at a time, newest by default, with every release beside it. */
export function PublicChangelog(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const { version } = useParams();
  const base = `/public/${project.slug}/changelog`;
  const index = useApi<PublicReleaseIndex>(base);
  const one = useApi<PublicRelease>(`${base}/${encodeURIComponent(version ?? "latest")}`);
  const [more, setMore] = useState<ReleaseHead[]>([]);
  const [older, setOlder] = useState<string | null>(null);
  useEffect(() => {
    setOlder(index.data?.older ?? null);
  }, [index.data]);
  useEffect(() => {
    window.scrollTo({ top: 0 });
  }, [version]);
  const loadOlder = async (): Promise<void> => {
    if (!older) return;
    const next = await api<PublicReleaseIndex>(`${base}?before=${encodeURIComponent(older)}`);
    setMore((m) => [...m, ...next.releases]);
    setOlder(next.older);
  };
  const releases = [...(index.data?.releases ?? []), ...more];
  const r = one.data;
  return (
    <>
      <PageHeader title={`${project.name} changelog`} />
      {index.data && releases.length === 0 ? (
        <p className="text-muted-foreground">No release is published yet.</p>
      ) : (
        <div className="grid gap-8 lg:grid-cols-[13rem_minmax(0,1fr)]">
          <VersionIndex slug={project.slug} releases={releases} open={r?.release.version ?? ""} older={older !== null} onOlder={() => void loadOlder()} />
          <div className="flex min-w-0 flex-col gap-6">
            {one.error?.status === 404 ? (
              <p className="text-muted-foreground">This release does not exist.</p>
            ) : !r ? (
              <Skeleton className="h-72 rounded-xl" />
            ) : (
              <>
                <ReleaseCard release={r.release} />
                <div className="grid gap-3 sm:grid-cols-2">
                  <Neighbour slug={project.slug} release={r.newer} side="newer" />
                  <Neighbour slug={project.slug} release={r.older} side="older" />
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </>
  );
}
