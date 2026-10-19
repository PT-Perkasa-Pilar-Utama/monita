import { expect, test } from "bun:test";
import { inlineClasses } from "./brandSvg";

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
