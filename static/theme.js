// Applies the theme the viewer chose before the first paint; app.js holds the switch.
try {
  const theme = localStorage.getItem("monita-theme");
  if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
} catch {}
