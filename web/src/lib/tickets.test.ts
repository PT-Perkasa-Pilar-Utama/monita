import { expect, test } from "bun:test";
import { dropOutcome, moveOnBoard } from "@/lib/tickets";
import type { TicketBoard, TicketRow } from "@/lib/types";

test("a drop on the same column does nothing", () => {
  expect(dropOutcome("backlog", "backlog")).toBe("none");
  expect(dropOutcome("done", "done")).toBe("none");
});

test("a drop on Done asks for a close reason first", () => {
  expect(dropOutcome("triage", "done")).toBe("ask_reason");
  expect(dropOutcome("in_progress", "done")).toBe("ask_reason");
});

test("a drop on any other column moves the ticket", () => {
  expect(dropOutcome("triage", "backlog")).toBe("move");
  expect(dropOutcome("done", "in_progress")).toBe("move");
});

const row = (number: number, stage: TicketRow["stage"]): TicketRow => ({
  number,
  title: `Ticket ${number}`,
  public_title: null,
  description: "x",
  page_url: null,
  stage,
  close_reason: stage === "done" ? "fixed" : null,
  created_at: 10,
  updated_at: 10,
  ref_revoked: false,
});
const board: TicketBoard = { tickets: [row(1, "triage"), row(2, "backlog"), row(3, "done")], counts: { triage: 1, backlog: 1, in_progress: 0, done: 1 } };

test("a moved ticket tops its new column and both counts change", () => {
  const b = moveOnBoard(board, 1, "in_progress", null, 99);
  expect(b.tickets[0]).toMatchObject({ number: 1, stage: "in_progress", updated_at: 99 });
  expect(b.counts).toEqual({ triage: 0, backlog: 1, in_progress: 1, done: 1 });
});

test("Done keeps the reason and leaving Done drops it", () => {
  expect(moveOnBoard(board, 2, "done", "duplicate", 99).tickets[0]?.close_reason).toBe("duplicate");
  expect(moveOnBoard(board, 3, "backlog", "fixed", 99).tickets[0]?.close_reason).toBeNull();
});

test("an unknown ticket or the same column leaves the board as it is", () => {
  expect(moveOnBoard(board, 42, "done", "fixed", 99)).toBe(board);
  expect(moveOnBoard(board, 2, "backlog", null, 99)).toBe(board);
});
