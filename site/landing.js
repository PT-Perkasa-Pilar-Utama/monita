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

  // ---------- Hero: a 3D field of daily uptime bars ----------
  const field = document.querySelector("[data-field]");
  if (field) {
    const ctx = field.getContext("2d");
    const ROWS = 8;
    const COLS = 32;
    // Height is a day's mean response time; a few days had downtime.
    const bars = [];
    for (let r = 0; r < ROWS; r++) {
      for (let c = 0; c < COLS; c++) {
        const wave = 0.35 + 0.18 * Math.sin(c / 4 + r * 1.3) + 0.1 * Math.sin(c / 1.7 + r);
        const down = (r * 31 + c * 17) % 97 === 0 || (r === 4 && c === COLS - 3);
        bars.push({ r, c, h: down ? 1.6 : Math.max(0.12, wave), down });
      }
    }
    let tiltX = 0;
    let tiltY = 0;
    field.addEventListener("pointermove", (e) => {
      const b = field.getBoundingClientRect();
      tiltX = ((e.clientX - b.left) / b.width - 0.5) * 0.5;
      tiltY = ((e.clientY - b.top) / b.height - 0.5) * 0.3;
    });
    field.addEventListener("pointerleave", () => {
      tiltX = 0;
      tiltY = 0;
    });
    const size = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const b = field.getBoundingClientRect();
      field.width = Math.round(b.width * dpr);
      field.height = Math.round(b.height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    size();
    window.addEventListener("resize", size);
    let yaw = -0.55;
    let pitch = 0.62;
    const draw = (t) => {
      const w = field.clientWidth;
      const h = field.clientHeight;
      ctx.clearRect(0, 0, w, h);
      const time = t / 1000;
      yaw += ((-0.55 + Math.sin(time * 0.18) * 0.32 + tiltX) - yaw) * 0.04;
      pitch += ((0.62 + tiltY) - pitch) * 0.04;
      const scan = (time * 6) % COLS;
      const cy = Math.cos(yaw);
      const sy = Math.sin(yaw);
      const cp = Math.cos(pitch);
      const sp = Math.sin(pitch);
      const scale = Math.min(w / 50, h / 34);
      const project = (x, y, z) => {
        const X = x * cy - z * sy;
        const Z = x * sy + z * cy;
        const Y = y * cp - Z * sp;
        const D = y * sp + Z * cp + 70;
        const k = (70 / D) * scale;
        return [w / 2 + X * k, h * 0.62 - Y * k, D];
      };
      const items = bars.map((b) => {
        const x = (b.c - COLS / 2) * 1.15;
        const z = (b.r - ROWS / 2) * 2.1;
        const near = Math.max(0, 1 - Math.abs(b.c - scan) / 2.5);
        const grow = still ? 1 : 0.85 + 0.15 * Math.sin(time * 1.4 + b.c * 0.3 + b.r);
        return { b, x, z, hh: b.h * 6 * grow + near * 1.2, near, depth: project(x, 0, z)[2] };
      });
      items.sort((a, b) => b.depth - a.depth);
      const face = (pts, fill) => {
        ctx.beginPath();
        pts.forEach(([px, py], i) => (i ? ctx.lineTo(px, py) : ctx.moveTo(px, py)));
        ctx.closePath();
        ctx.fillStyle = fill;
        ctx.fill();
      };
      for (const { b, x, z, hh, near } of items) {
        const base = b.down ? palette.down : palette.up;
        // The scan line is the latest round of checks passing over each service.
        const lift = 1 + near * 0.55;
        const s = 0.38;
        const p = (dx, y, dz) => project(x + dx, y, z + dz);
        const fog = 0.55 + 0.45 * (1 - (z + ROWS) / (ROWS * 2.2));
        face([p(-s, hh, -s), p(s, hh, -s), p(s, hh, s), p(-s, hh, s)], rgb(base, 1.15 * lift, fog));
        face([p(-s, 0, s), p(s, 0, s), p(s, hh, s), p(-s, hh, s)], rgb(base, 0.72 * lift, fog));
        face([p(s, 0, -s), p(s, 0, s), p(s, hh, s), p(s, hh, -s)], rgb(base, 0.52 * lift, fog));
      }
    };
    whileVisible(field, draw);
  }

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

  // ---------- Screens stacked in 3D, tilting with the pointer ----------
  const stack = document.querySelector("[data-stack]");
  if (stack && !still) {
    stack.addEventListener("pointermove", (e) => {
      const b = stack.getBoundingClientRect();
      stack.style.setProperty("--rx", `${(0.5 - (e.clientY - b.top) / b.height) * 14}deg`);
      stack.style.setProperty("--ry", `${((e.clientX - b.left) / b.width - 0.5) * 22}deg`);
    });
    stack.addEventListener("pointerleave", () => {
      stack.style.removeProperty("--rx");
      stack.style.removeProperty("--ry");
    });
  }

  // ---------- Install commands typed into a terminal ----------
  const term = document.querySelector("[data-type]");
  if (term && !still) {
    const lines = [...term.querySelectorAll("[data-line]")];
    const full = lines.map((l) => l.textContent);
    lines.forEach((l) => (l.textContent = ""));
    let started = false;
    new IntersectionObserver(([e]) => {
      if (!e.isIntersecting || started) return;
      started = true;
      let li = 0;
      let ch = 0;
      const tick = () => {
        if (li >= lines.length) return;
        lines[li].textContent = full[li].slice(0, ++ch);
        if (ch >= full[li].length) {
          li++;
          ch = 0;
          setTimeout(tick, 380);
        } else setTimeout(tick, 24);
      };
      tick();
    }, { threshold: 0.6 }).observe(term);
  }

  // ---------- Sections rise into view ----------
  const reveal = document.querySelectorAll("[data-reveal]");
  if (still) reveal.forEach((el) => el.classList.add("is-in"));
  else {
    const io = new IntersectionObserver((entries) => {
      for (const e of entries) if (e.isIntersecting) {
        e.target.classList.add("is-in");
        io.unobserve(e.target);
      }
    }, { threshold: 0.15 });
    reveal.forEach((el) => io.observe(el));
  }
})();
