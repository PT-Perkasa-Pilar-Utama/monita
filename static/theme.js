// Applies the theme before the first paint: the saved choice, or the OS setting. The React pages
// read the `dark` class; the remaining server pages read data-theme.
try {
  const theme = localStorage.getItem("monita-theme");
  const dark = theme ? theme === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.add(dark ? "dark" : "light");
  if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
} catch {}
