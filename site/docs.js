// Docs pages: filter the sidebar as you type ("/" focuses the box), and mark the section in view.
(() => {
  const box = document.querySelector("[data-docs-search]");
  const empty = document.querySelector(".side__empty");
  if (box) {
    box.addEventListener("input", () => {
      const q = box.value.trim().toLowerCase();
      let shown = 0;
      for (const group of document.querySelectorAll(".side__group")) {
        let any = false;
        for (const a of group.querySelectorAll("a[data-terms]")) {
          const hit = q === "" || a.dataset.terms.includes(q);
          a.parentElement.hidden = !hit;
          any ||= hit;
          if (hit) shown++;
        }
        group.hidden = !any;
      }
      if (empty) empty.hidden = shown > 0;
    });
    box.addEventListener("keydown", (e) => {
      if (e.key !== "Enter") return;
      const first = document.querySelector(".side__group:not([hidden]) li:not([hidden]) a");
      if (first) location.href = first.href;
    });
    document.addEventListener("keydown", (e) => {
      const typing = e.target instanceof HTMLElement && e.target.matches("input, textarea");
      if ((e.key === "/" && !typing) || (e.key === "k" && (e.metaKey || e.ctrlKey))) {
        e.preventDefault();
        box.focus();
      }
    });
  }

  const links = [...document.querySelectorAll(".docs__toc a")];
  const heads = links.map((a) => document.getElementById(a.hash.slice(1))).filter(Boolean);
  if (heads.length === 0) return;
  const mark = () => {
    let current = heads[0];
    for (const h of heads) if (h.getBoundingClientRect().top < 120) current = h;
    for (const a of links) a.toggleAttribute("aria-current", a.hash === `#${current.id}`);
  };
  document.addEventListener("scroll", mark, { passive: true });
  mark();
})();
