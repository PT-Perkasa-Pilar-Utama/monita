// Applies the theme and the sidebar state the viewer chose before the first paint; app.js holds
// the switches.
try {
  const theme = localStorage.getItem("monita-theme");
  if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
  if (localStorage.getItem("monita-sidebar") === "collapsed") document.documentElement.dataset.sidebar = "collapsed";
} catch {}
