import type { ReactElement } from "react";
import { CodeCopy, CopyField } from "@/components/app/CopyField";
import { DetailPart, DetailsSheet } from "@/components/app/DetailsSheet";
import { Facts } from "@/components/app/Facts";
import { parseHeaderLines, whenItFails } from "@/lib/checks";
import { monitorCurl } from "@/lib/monitorCurl";
import { stamp } from "@/lib/time";
import type { MonitorView } from "@/lib/types";

/** A monitor's request, what a pass means, what a failure does, and a curl command to try it. */
export function MonitorDetails({ m }: { m: MonitorView }): ReactElement {
  const headers = parseHeaderLines(m.headers);
  return (
    <DetailsSheet title={m.name} description="What Monita sends, what counts as a pass, and what a failure does.">
      <DetailPart title="Request">
        <CopyField id="monitor-url" value={m.url} />
        <Facts
          rows={[
            ["Method", m.method],
            ["Headers", headers.length === 0 ? "None" : headers.map(([n]) => n).join(", ")],
            ["Request body", m.body ? `${m.body.length.toLocaleString()} characters` : "None"],
            ["Timeout", `${m.timeout_s} seconds`],
            ["Every", m.every],
          ]}
        />
      </DetailPart>
      <DetailPart title="Try it yourself">
        <p className="text-muted-foreground">This curl command sends the same request and prints the status code and the time.</p>
        <CodeCopy id="monitor-curl" title="Same request as the check" code={monitorCurl(m)} />
      </DetailPart>
      <DetailPart title="A check passes when">
        <Facts
          rows={[
            ["The status code matches", <span className="font-mono">{m.expected_status}</span>],
            ["The response body has", m.keyword ? <span className="font-mono">{m.keyword}</span> : "Anything"],
            ["The certificate", m.skip_tls_verify ? "May be self-signed" : "Must be valid"],
          ]}
        />
      </DetailPart>
      <DetailPart title="When it fails">
        <Facts
          rows={[
            ["Counts as down after", m.fails_to_open === 1 ? "1 failed check" : `${m.fails_to_open} failed checks in a row`],
            ["Then it", whenItFails(m.alert, m.auto_incident)],
            ["Status page", m.is_public ? "Shown, with its incidents" : "Not shown"],
          ]}
        />
      </DetailPart>
      <DetailPart title="Settings">
        <Facts
          rows={[
            ["Next check", stamp(m.next_check_at)],
            ["Created", stamp(m.created_at)],
          ]}
        />
      </DetailPart>
    </DetailsSheet>
  );
}
