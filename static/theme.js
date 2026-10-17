// Applies the theme before the first paint: the saved choice, or the OS setting. The app reads
// the `dark` class; the landing and docs site (site/app.css) read data-theme.
try {
  const theme = localStorage.getItem("monita-theme");
  const dark = theme ? theme === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.add(dark ? "dark" : "light");
  if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
} catch {}
