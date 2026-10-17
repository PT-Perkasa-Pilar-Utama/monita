// Copy buttons (<button data-copy="element-id">), the theme switch (<button data-theme-toggle>)
// and the sidebar switch (<button data-sidebar-toggle>, or Ctrl+B / Cmd+B). theme.js applies the
// saved theme and sidebar state before the first paint.
const toggleSidebar = () => {
  const root = document.documentElement;
  const collapsed = root.dataset.sidebar !== "collapsed";
  if (collapsed) root.dataset.sidebar = "collapsed";
  else delete root.dataset.sidebar;
  document.querySelectorAll("[data-sidebar-toggle]").forEach((b) => b.setAttribute("aria-expanded", String(!collapsed)));
  try {
    localStorage.setItem("monita-sidebar", collapsed ? "collapsed" : "expanded");
  } catch {}
};
document.addEventListener("click", (event) => {
  if (event.target.closest("[data-sidebar-toggle]")) toggleSidebar();
});
document.addEventListener("keydown", (event) => {
  if (event.key.toLowerCase() === "b" && (event.ctrlKey || event.metaKey) && !event.altKey && document.querySelector(".sidebar")) {
    event.preventDefault();
    toggleSidebar();
  }
});
document.querySelectorAll("[data-sidebar-toggle]").forEach((b) =>
  b.setAttribute("aria-expanded", String(document.documentElement.dataset.sidebar !== "collapsed")),
);

document.addEventListener("click", (event) => {
  if (!event.target.closest("[data-theme-toggle]")) return;
  const root = document.documentElement;
  const dark = root.dataset.theme
    ? root.dataset.theme === "dark"
    : window.matchMedia("(prefers-color-scheme: dark)").matches;
  root.dataset.theme = dark ? "light" : "dark";
  try {
    localStorage.setItem("monita-theme", root.dataset.theme);
  } catch {}
});

document.addEventListener("click", async (event) => {
  const button = event.target.closest("[data-copy]");
  if (!button) return;
  const source = document.getElementById(button.dataset.copy);
  if (!source) return;
  const label = (button.dataset.label ??= button.textContent);
  setTimeout(() => (button.textContent = label), 2000);
  try {
    await navigator.clipboard.writeText(source.textContent.trim());
    button.textContent = "Copied";
  } catch {
    const range = document.createRange();
    range.selectNodeContents(source);
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    button.textContent = "Press Ctrl+C";
  }
});
