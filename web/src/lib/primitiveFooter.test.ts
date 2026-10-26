import { expect, test } from "bun:test";
import { footerFor } from "@/lib/primitiveFooter";

const base = "/projects/payments";

test("each primitive's list, detail and form pages get its footer", () => {
  expect(footerFor(`${base}/monitors`, base)).toBe("monitors");
  expect(footerFor(`${base}/monitors/12/edit`, base)).toBe("monitors");
  expect(footerFor(`${base}/heartbeats/new`, base)).toBe("heartbeats");
  expect(footerFor(`${base}/incidents/4`, base)).toBe("incidents");
  expect(footerFor(`${base}/maintenance/new`, base)).toBe("maintenance");
  expect(footerFor(`${base}/tickets/done`, base)).toBe("tickets");
  expect(footerFor(`${base}/changelog`, base)).toBe("changelog");
});

test("Overview, Settings and look-alike paths get none", () => {
  expect(footerFor(base, base)).toBeNull();
  expect(footerFor(`${base}/settings/alerts`, base)).toBeNull();
  expect(footerFor(`${base}/monitorsx`, base)).toBeNull();
  expect(footerFor("/projects/payments-two/monitors", base)).toBeNull();
  // Same length as base, another project.
  expect(footerFor("/projects/receipts/monitors", base)).toBeNull();
});
