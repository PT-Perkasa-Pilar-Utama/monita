// The demo is a snapshot of the rendered pages, without the app's script. The theme switch still
// works; any other button or a link the demo cannot follow says so in the banner.
(() => {
  const banner = document.querySelector(".demo-banner");
  if (!banner) return;
  const original = banner.innerHTML;
  let timer;
  const flash = () => {
    banner.innerHTML =
      '<strong>Demo</strong> Read-only: this action needs a running Monita. <a href="https://github.com/PT-Perkasa-Pilar-Utama/monita#install">Install it</a>.';
    banner.classList.add("is-flash");
    clearTimeout(timer);
    timer = setTimeout(() => {
      banner.innerHTML = original;
      banner.classList.remove("is-flash");
    }, 3500);
  };
  document.addEventListener("click", (e) => {
    if (!(e.target instanceof Element)) return;
    const theme = e.target.closest('button[aria-label^="Switch to the"]');
    if (theme) {
      const dark = document.documentElement.classList.toggle("dark");
      document.documentElement.classList.toggle("light", !dark);
      try {
        localStorage.setItem("monita-theme", dark ? "dark" : "light");
      } catch {}
      return;
    }
    if (e.target.closest('a[href="#demo-off"], button')) {
      e.preventDefault();
      flash();
    }
  });
  document.addEventListener("submit", (e) => {
    e.preventDefault();
    flash();
  });
})();
