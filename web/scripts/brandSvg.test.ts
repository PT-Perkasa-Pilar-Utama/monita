import { expect, test } from "bun:test";
import { inlineClasses, THEMED_COLORS } from "./brandSvg";

test("a token class becomes a fixed colour, with its opacity", () => {
  expect(inlineClasses('<circle class="fill-mascot-body stroke-primary/60"/>')).toBe('<circle style="fill:#fbe4f2;stroke:#c4258f;stroke-opacity:0.6"/>');
});

test("motion classes become inline animation, and a second style attribute merges", () => {
  const out = inlineClasses('<g class="animate-[monita-beat_2s_ease-in-out_infinite]" style="animation-delay:1s"/>');
  expect(out).toBe('<g style="animation:monita-beat 2s ease-in-out infinite;animation-delay:1s"/>');
});

test("an unknown class stops the build instead of shipping art with a colour missing", () => {
  expect(() => inlineClasses('<rect class="fill-down"/>')).toThrow(/fill-down/);
});

test("themed art reads its colours from the page, and Moni keeps hers", () => {
  const out = inlineClasses('<rect class="fill-card stroke-up/70"/><circle class="fill-mascot-body"/>', THEMED_COLORS);
  expect(out).toBe('<rect style="fill:var(--art-card,#ffffff);stroke:var(--art-up,#23784e);stroke-opacity:0.7"/><circle style="fill:#fbe4f2"/>');
});

test("a label's type classes become font styles", () => {
  expect(inlineClasses('<text class="font-mono text-[11px] font-semibold"/>')).toBe(
    "<text style=\"font-family:'Geist Mono',ui-monospace,monospace;font-size:11px;font-weight:600\"/>",
  );
});
