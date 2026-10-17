// The demo is a snapshot: a form or a filtered list has nothing behind it. Say so instead.
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
    if (e.target instanceof Element && e.target.closest('a[href="#demo-off"]')) {
      e.preventDefault();
      flash();
    }
  });
  document.addEventListener("submit", (e) => {
    e.preventDefault();
    flash();
  });
})();
