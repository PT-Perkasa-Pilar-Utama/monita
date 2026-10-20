// The landing page's motion. Every animation shows a real Monita behaviour. Colours come from the
// page's CSS tokens, so the theme switch recolours them. With prefers-reduced-motion each one draws
// its first frame and stops.
(() => {
  const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const root = document.documentElement;

  // ---------- Theme colours, re-read when the theme changes ----------
  let palette = {};
  const hex = (v) => {
    const m = /^#?([0-9a-f]{6})$/i.exec(v.trim());
    if (!m) return [128, 128, 128];
    const n = parseInt(m[1], 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  };
  const readPalette = () => {
    const css = getComputedStyle(root);
    palette = {
      up: hex(css.getPropertyValue("--green")),
      down: hex(css.getPropertyValue("--red")),
      accent: hex(css.getPropertyValue("--accent")),
      bg: hex(css.getPropertyValue("--bg")),
      fg: hex(css.getPropertyValue("--fg")),
    };
  };
  readPalette();
  new MutationObserver(readPalette).observe(root, { attributes: true, attributeFilter: ["data-theme"] });
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", readPalette);
  const rgb = (c, k = 1, a = 1) => `rgba(${c.map((v) => Math.round(Math.min(255, v * k))).join(",")},${a})`;

  // Runs `step` on every frame while `el` is on screen.
  const whileVisible = (el, step) => {
    let on = false;
    let raf = 0;
    const loop = (t) => {
      step(t);
      if (on) raf = requestAnimationFrame(loop);
    };
    new IntersectionObserver(([e]) => {
      on = e.isIntersecting && !still;
      cancelAnimationFrame(raf);
      if (on) raf = requestAnimationFrame(loop);
    }).observe(el);
    step(0);
  };

  // ---------- A failing check, step by step ----------
  const sim = document.querySelector("[data-sim]");
  if (sim) {
    const log = sim.querySelector("[data-log]");
    const steps = [...sim.querySelectorAll("[data-step]")];
    // [seconds into the run, request line, status, step to light]
    const script = [
      [0, "GET /healthz", "200 in 84 ms", -1],
      [1.2, "GET /healthz", "200 in 91 ms", -1],
      [2.4, "GET /healthz", "503 Service Unavailable", 0],
      [3.6, "GET /healthz", "503 Service Unavailable", 1],
      [4.4, "incident opened", "API health is down", 2],
      [5.4, "email and webhook sent", "oncall@company.co.id", 3],
      [7.2, "GET /healthz", "200 in 102 ms", 4],
    ];
    const LOOP = 11;
    let last = -1;
    const render = (upTo) => {
      log.replaceChildren(
        ...script.slice(0, upTo + 1).map(([, req, res], i) => {
          const li = document.createElement("li");
          const bad = /503|down/.test(res);
          li.className = bad ? "is-bad" : i === script.length - 1 ? "is-good" : "";
          li.innerHTML = `<span>${req}</span><span>${res}</span>`;
          return li;
        }),
      );
      const lit = Math.max(...script.slice(0, upTo + 1).map((s) => s[3]));
      steps.forEach((el, i) => el.classList.toggle("is-on", i <= lit));
    };
    if (still) render(script.length - 1);
    else
      whileVisible(sim, (t) => {
        const at = ((t / 1000) % LOOP);
        let i = -1;
        script.forEach((s, n) => {
          if (at >= s[0]) i = n;
        });
        if (i !== last) {
          last = i;
          render(Math.max(0, i));
        }
      });
  }

  // ---------- Heartbeats: pings on a track, one goes missing ----------
  const track = document.querySelector("[data-track]");
  if (track) {
    const ctx = track.getContext("2d");
    const size = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      track.width = Math.round(track.clientWidth * dpr);
      track.height = Math.round(track.clientHeight * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    size();
    window.addEventListener("resize", size);
    const label = document.querySelector("[data-track-state]");
    // The job pings once a second. In each 12 s cycle it stops for three pings, the 0.6 s grace
    // runs out, and the next ping that arrives closes the incident.
    const CYCLE = 12;
    const GONE = [6, 7, 8];
    const GRACE = 0.6;
    whileVisible(track, (t) => {
      const w = track.clientWidth;
      const h = track.clientHeight;
      const time = still ? CYCLE + 8 : CYCLE + t / 1000;
      const speed = w / 10;
      const xAt = (s) => w - (time - s) * speed;
      ctx.clearRect(0, 0, w, h);
      const mid = h / 2;
      ctx.strokeStyle = rgb(palette.fg, 1, 0.15);
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(0, mid);
      ctx.lineTo(w, mid);
      ctx.stroke();
      for (let c = Math.floor((time - 10) / CYCLE); c <= Math.floor(time / CYCLE); c++) {
        const from = c * CYCLE + GONE[0] + GRACE;
        const to = Math.min(time, c * CYCLE + GONE[GONE.length - 1] + 1);
        if (from > time) continue;
        ctx.fillStyle = rgb(palette.down, 1, 0.16);
        ctx.fillRect(xAt(from), 0, xAt(to) - xAt(from), h);
        ctx.fillStyle = rgb(palette.down, 1, 1);
        ctx.fillRect(xAt(from) - 1, 6, 2, h - 12);
      }
      for (let n = Math.ceil(time - 10); n <= time; n++) {
        if (GONE.includes(n % CYCLE)) continue;
        const x = xAt(n);
        const age = time - n;
        ctx.fillStyle = rgb(palette.up, 1, 1);
        ctx.beginPath();
        ctx.roundRect(x - 3, mid - 16, 6, 32, 3);
        ctx.fill();
        if (age < 0.4) {
          ctx.strokeStyle = rgb(palette.up, 1, 1 - age / 0.4);
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.arc(x, mid, 10 + age * 60, 0, Math.PI * 2);
          ctx.stroke();
        }
      }
      const at = time % CYCLE;
      const missed = at >= GONE[0] + GRACE && at < GONE[GONE.length - 1] + 1;
      const back = at >= GONE[GONE.length - 1] + 1 && at < GONE[GONE.length - 1] + 3;
      if (label) {
        const text = missed
          ? "Deadline passed: incident opened"
          : back
            ? "Ping received: incident resolved"
            : "Pings arriving on time";
        if (label.textContent !== text) {
          label.textContent = text;
          label.classList.toggle("is-bad", missed);
        }
      }
    });
  }

  // ---------- Hero screenshot: tilted back, it lies flat as it scrolls up ----------
  const shot = document.querySelector("[data-tilt]");
  if (shot && !still) {
    let queued = false;
    const tilt = () => {
      queued = false;
      const top = shot.getBoundingClientRect().top;
      const flat = Math.min(1, Math.max(0, 1 - (top - window.innerHeight * 0.25) / (window.innerHeight * 0.6)));
      shot.style.setProperty("--tilt", `${(1 - flat) * 24}deg`);
    };
    window.addEventListener("scroll", () => {
      if (!queued) requestAnimationFrame(tilt);
      queued = true;
    }, { passive: true });
    tilt();
  }

  // ---------- Tabs: Install, and Connections, which also lights the matching line on the map ----------
  document.querySelectorAll("[data-tabs]").forEach((tabs) => {
    const buttons = [...tabs.querySelectorAll('[role="tab"]')];
    const select = (i, focus) => {
      buttons.forEach((b, n) => {
        b.setAttribute("aria-selected", String(n === i));
        b.tabIndex = n === i ? 0 : -1;
        document.getElementById(b.getAttribute("aria-controls")).hidden = n !== i;
      });
      tabs.querySelectorAll("[data-link], [data-node]").forEach((el) => {
        el.classList.toggle("is-on", Number(el.dataset.link ?? el.dataset.node) === i);
      });
      if (focus) buttons[i].focus();
      window.dispatchEvent(new Event("resize"));
    };
    buttons.forEach((b, i) => {
      b.addEventListener("click", () => select(i, false));
      b.addEventListener("keydown", (e) => {
        const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
        if (!step) return;
        e.preventDefault();
        select((i + step + buttons.length) % buttons.length, true);
      });
    });
    select(0, false);
  });

  // ---------- Screenshots open at full size in a dialog ----------
  const dialog = document.querySelector("[data-zoom-dialog]");
  if (dialog) {
    const shots = [...document.querySelectorAll("[data-zoom]")].sort((a, b) => a.dataset.zoom - b.dataset.zoom);
    const img = dialog.querySelector("img");
    const caption = dialog.querySelector("p");
    let at = 0;
    const show = (i) => {
      at = (i + shots.length) % shots.length;
      const source = shots[at].querySelector("img");
      img.src = source.currentSrc || source.src;
      img.alt = source.alt;
      caption.textContent = source.alt;
    };
    shots.forEach((b, i) => b.addEventListener("click", () => {
      show(i);
      dialog.showModal();
    }));
    dialog.addEventListener("click", (e) => {
      const step = e.target.closest("[data-zoom-step]");
      if (step) show(at + Number(step.dataset.zoomStep));
      else if (e.target === dialog || e.target.closest("[data-zoom-close]")) dialog.close();
    });
    dialog.addEventListener("keydown", (e) => {
      if (e.key === "ArrowRight") show(at + 1);
      if (e.key === "ArrowLeft") show(at - 1);
    });
  }

})();
