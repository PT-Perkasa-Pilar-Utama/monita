import type { ReactElement } from "react";
import { CodeCopy, CopyField } from "@/components/app/CopyField";
import { DetailPart, DetailsSheet } from "@/components/app/DetailsSheet";
import { Facts } from "@/components/app/Facts";
import { certWords, parseHeaderLines, signInWords, watchesCert, whenItFails } from "@/lib/checks";
import { monitorCommand } from "@/lib/monitorCurl";
import { useCanEdit } from "@/lib/session";
import { nowSecs, stamp } from "@/lib/time";
import type { MonitorView } from "@/lib/types";

/** A monitor's request, what a pass means, what a failure does, and a command to try it. */
export function MonitorDetails({ m }: { m: MonitorView }): ReactElement {
  return (
    <DetailsSheet title={m.name} description="What Monita checks, what counts as a pass, and what a failure does.">
      {m.kind === "http" ? <HttpParts m={m} /> : <PlainParts m={m} />}
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

/** A TCP or DNS check: its target, a command to try it, and what a pass means. */
function PlainParts({ m }: { m: MonitorView }): ReactElement {
  const tcp = m.kind === "tcp";
  return (
    <>
      <DetailPart title={tcp ? "TCP port" : "DNS name"}>
        <CopyField id="monitor-url" value={m.url} />
        <Facts
          rows={[
            ["Timeout", `${m.timeout_s} seconds`],
            ["Every", m.every],
          ]}
        />
      </DetailPart>
      <DetailPart title="Try it yourself">
        <p className="text-muted-foreground">{tcp ? "This command opens a connection to the same port." : "This command asks the server's resolver, as the check does."}</p>
        <CodeCopy id="monitor-curl" title="Same check as Monita" code={monitorCommand(m)} />
      </DetailPart>
      <DetailPart title="A check passes when">
        <Facts
          rows={
            tcp
              ? [["The port", "Accepts a connection"]]
              : [["The name resolves to", m.expected_address ? <span className="font-mono">{m.expected_address}</span> : "Any address"]]
          }
        />
      </DetailPart>
    </>
  );
}

/** An HTTP check: the request, a curl command, and what a pass means. */
function HttpParts({ m }: { m: MonitorView }): ReactElement {
  const headers = parseHeaderLines(m.headers);
  const canEdit = useCanEdit();
  return (
    <>
      <DetailPart title="Request">
        <CopyField id="monitor-url" value={m.url} />
        <Facts
          rows={[
            ["Method", m.method],
            ["Sign-in", signInWords(m)],
            ["Headers", headers.length === 0 ? "None" : headers.map(([n]) => n).join(", ")],
            ["Request body", !m.body ? "None" : canEdit ? `${m.body.length.toLocaleString()} characters` : "Sent, hidden"],
            ["Timeout", `${m.timeout_s} seconds`],
            ["Every", m.every],
          ]}
        />
        {!canEdit && (
          <p className="text-muted-foreground">
            As a supervisor you see header names but not their values. The request body, the sign-in name and anything after <code>?</code> in the URL read (hidden), because they can hold credentials.
          </p>
        )}
      </DetailPart>
      <DetailPart title="Try it yourself">
        <p className="text-muted-foreground">
          This curl command sends the same request and prints the status code and the time.{m.auth_kind !== "none" && " Set the secret's value in your shell first, as its first line shows."}
        </p>
        <CodeCopy id="monitor-curl" title="Same request as the check" code={monitorCommand(m)} />
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
      {watchesCert(m.kind, m.url) && (
        <DetailPart title="Certificate">
          <Facts
            rows={[
              ["Expires", m.cert_expires_at ? certWords(m.cert_expires_at, nowSecs()) : "Not seen yet"],
              ["Warning", m.cert_warn_days ? `${m.cert_warn_days} days before, once per certificate` : "Off"],
            ]}
          />
        </DetailPart>
      )}
    </>
  );
}
