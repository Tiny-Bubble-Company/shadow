// Shadow activity dashboard. Talks only to this origin (/shadow/activity*); no
// third-party code. `?demo=1` renders synthetic data, `?theme=dark|light` forces a theme,
// `?embed=1` trims chrome for the tray popover, `?w=1h|24h|7d|30d` picks the window.
(function () {
  "use strict";

  var params = new URLSearchParams(location.search);
  var DEMO = params.has("demo") && params.get("demo") !== "0";
  var EMBED = params.has("embed") && params.get("embed") !== "0";
  var WINDOWS = ["1h", "24h", "7d", "30d"];
  var REFRESH_MS = 5000;
  var theme = params.get("theme");
  if (theme === "dark" || theme === "light") document.documentElement.setAttribute("data-theme", theme);
  if (EMBED) document.body.classList.add("embed");

  var state = { window: pickWindow(), summary: null, incidents: [], online: null, timer: null };

  function pickWindow() {
    var w = params.get("w");
    if (WINDOWS.indexOf(w) >= 0) return w;
    try {
      var saved = localStorage.getItem("shadow.window");
      if (WINDOWS.indexOf(saved) >= 0) return saved;
    } catch (e) { /* storage unavailable */ }
    return "24h";
  }

  // ---------------------------------------------------------------- DOM helpers
  var SVGNS = "http://www.w3.org/2000/svg";
  function $(id) { return document.getElementById(id); }
  function h(tag, attrs, kids) {
    var el = document.createElement(tag);
    setAttrs(el, attrs);
    append(el, kids);
    return el;
  }
  function s(tag, attrs, kids) {
    var el = document.createElementNS(SVGNS, tag);
    setAttrs(el, attrs);
    append(el, kids);
    return el;
  }
  function setAttrs(el, attrs) {
    if (!attrs) return;
    Object.keys(attrs).forEach(function (k) {
      var v = attrs[k];
      if (v === null || v === undefined || v === false) return;
      if (k === "class") el.setAttribute("class", v);
      else if (k === "text") el.textContent = v;
      else if (k === "style") Object.keys(v).forEach(function (p) { el.style[p] = v[p]; });
      else if (k.slice(0, 2) === "on") el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v);
    });
  }
  function append(el, kids) {
    if (kids === null || kids === undefined) return;
    if (!Array.isArray(kids)) kids = [kids];
    kids.forEach(function (k) {
      if (k === null || k === undefined || k === false) return;
      el.appendChild(typeof k === "string" || typeof k === "number" ? document.createTextNode(String(k)) : k);
    });
  }
  function clear(el) { while (el.firstChild) el.removeChild(el.firstChild); return el; }

  // ---------------------------------------------------------------- formatting
  var nf = new Intl.NumberFormat("en");
  var cf = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });
  function fmt(n) { n = n || 0; return n >= 100000 ? cf.format(n) : nf.format(n); }
  function pct(a, b) { return b ? Math.round((100 * a) / b) + "%" : "0%"; }
  function plural(n, one, many) { return fmt(n) + " " + (n === 1 ? one : (many || one + "s")); }
  function bytes(n) {
    if (!n) return "0 B";
    var u = ["B", "KB", "MB", "GB"], i = 0;
    while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
    return (i ? n.toFixed(1) : n) + " " + u[i];
  }
  function ago(ts) {
    var d = Math.max(0, (Date.now() - ts) / 1000);
    if (d < 45) return "just now";
    if (d < 3600) return Math.round(d / 60) + "m ago";
    if (d < 86400) return Math.round(d / 3600) + "h ago";
    return new Date(ts).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  function clock(ts, withDate) {
    var d = new Date(ts);
    var t = d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
    return withDate ? d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" }) + ", " + t : t;
  }

  var KIND_LABELS = {
    email: "Email addresses", phone: "Phone numbers", person: "Names", org: "Organisations",
    term: "Protected terms", address: "Postal addresses", iban: "IBANs", credit_card: "Card numbers",
    national_id: "National IDs", ip: "IP addresses", mac: "MAC addresses", hostname: "Hostnames",
    secret: "Secrets & API keys", username: "Usernames", identifier: "User identifiers"
  };
  var KIND_SHORT = {
    email: "Email", phone: "Phone", person: "Name", org: "Org", term: "Term", address: "Address",
    iban: "IBAN", credit_card: "Card", national_id: "National ID", ip: "IP", mac: "MAC",
    hostname: "Host", secret: "Secret", username: "Username", identifier: "User ID"
  };
  var REASON_LABELS = {
    image_blocked: "Image blocked by policy", pdf_blocked: "PDF blocked by policy",
    unreadable_pdf: "PDF could not be read", file_too_large: "File over size limit", file_policy: "File policy"
  };
  var PROVIDERS = {
    openai: ["OpenAI", "#10a37f", "AI"], chatgpt: ["ChatGPT", "#0f8f6e", "GP"], anthropic: ["Anthropic", "#d97757", "A\\"],
    claude: ["Claude.ai", "#c96442", "C"], gemini: ["Google Gemini", "#4f86f7", "G"], vertex: ["Google Vertex AI", "#3b78e7", "V"],
    mistral: ["Mistral", "#f7a52b", "M"], openrouter: ["OpenRouter", "#6467f2", "OR"], azure: ["Azure OpenAI", "#2f8fe0", "Az"],
    bedrock: ["AWS Bedrock", "#f29b25", "B"], groq: ["Groq", "#f55036", "Gq"], deepseek: ["DeepSeek", "#4d6bfe", "D"],
    xai: ["xAI", "#444", "x"], cohere: ["Cohere", "#39594d", "Co"], perplexity: ["Perplexity", "#20808d", "P"],
    copilot: ["GitHub Copilot", "#6e40c9", "GH"], local: ["Local model", "#8a8aa3", "L"]
  };
  var PALETTE = ["#7c6cff", "#1fae8a", "#f5a524", "#ef4f6b", "#4f9cf7", "#b57cf6", "#8a8aa3"];
  function providerName(key, label) { return (PROVIDERS[key] && PROVIDERS[key][0]) || label || key; }
  function providerColor(key, i) { return (PROVIDERS[key] && PROVIDERS[key][1]) || PALETTE[i % PALETTE.length]; }
  function monogram(key, label) {
    var p = PROVIDERS[key];
    if (p) return p[2];
    return (label || key || "?").replace(/[^A-Za-z0-9]/g, "").slice(0, 2) || "?";
  }

  // Feather-style 24px icons drawn with strokes only.
  var ICONS = {
    email: ["M4 6h16v12H4z", "M4 7l8 6 8-6"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6"],
    secret: ["M14.5 9.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0z", "M13.2 12.8L21 20.5", "M18 17.5l2-2"],
    phone: ["M7 3h10v18H7z", "M11 18h2"],
    hostname: ["M4 5h16v6H4z", "M4 13h16v6H4z", "M8 8h.01", "M8 16h.01"],
    ip: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M3 12h18", "M12 3c2.5 2.5 3.5 5.5 3.5 9s-1 6.5-3.5 9c-2.5-2.5-3.5-5.5-3.5-9s1-6.5 3.5-9z"],
    credit_card: ["M3 6h18v12H3z", "M3 10h18", "M7 15h3"],
    iban: ["M3 10l9-6 9 6", "M5 10v8", "M10 10v8", "M14 10v8", "M19 10v8", "M3 20h18"],
    address: ["M12 21s-7-6.2-7-11.5a7 7 0 0 1 14 0C19 14.8 12 21 12 21z", "M12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z"],
    org: ["M5 21V4h9v17", "M14 9h5v12", "M8 8h3", "M8 12h3", "M8 16h3", "M3 21h18"],
    term: ["M3 12V4h8l10 10-8 8L3 12z", "M7.5 8h.01"],
    username: ["M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0z", "M16 12v1.5a2.5 2.5 0 0 0 5 0V12a9 9 0 1 0-3.5 7.1"],
    national_id: ["M3 5h18v14H3z", "M9 11a2 2 0 1 0 0-4 2 2 0 0 0 0 4z", "M5.5 16c.6-1.8 2-2.8 3.5-2.8s2.9 1 3.5 2.8", "M15 9h3", "M15 13h3"],
    mac: ["M7 7h10v10H7z", "M10 3v4", "M14 3v4", "M10 17v4", "M14 17v4", "M3 10h4", "M3 14h4", "M17 10h4", "M17 14h4"],
    identifier: ["M12 3a7 7 0 0 0-7 7v4", "M19 14v-4a7 7 0 0 0-3-5.7", "M12 7a3 3 0 0 0-3 3v5a8 8 0 0 0 1 4", "M15 10v4a11 11 0 0 1-1 5", "M12 11v4"]
  };
  function kindIcon(kind) {
    var paths = ICONS[kind] || ICONS.term;
    return h("span", { class: "kind-ico" }, s("svg", { viewBox: "0 0 24 24", "aria-hidden": "true" },
      paths.map(function (d) { return s("path", { d: d }); })));
  }
  function kindTags(kinds, limit) {
    var keys = Object.keys(kinds || {}).sort(function (a, b) { return kinds[b] - kinds[a]; });
    var tags = keys.slice(0, limit || 4).map(function (k) {
      return h("span", { class: "tag", text: kinds[k] + " " + (KIND_SHORT[k] || k) });
    });
    if (keys.length > (limit || 4)) tags.push(h("span", { class: "tag", text: "+" + (keys.length - (limit || 4)) }));
    return tags;
  }

  // ---------------------------------------------------------------- data
  function fetchJSON(url) {
    return fetch(url, { cache: "no-store", credentials: "omit" }).then(function (r) {
      if (!r.ok) throw new Error("HTTP " + r.status);
      return r.json();
    });
  }

  function load() {
    if (DEMO) {
      var d = demoData(state.window);
      setOnline(true);
      render(d.summary, d.incidents);
      return;
    }
    Promise.all([
      fetchJSON("/shadow/activity/summary?window=" + encodeURIComponent(state.window)),
      fetchJSON("/shadow/activity?action=incidents&limit=60")
    ]).then(function (res) {
      setOnline(true);
      $("disabled").hidden = res[0].enabled !== false;
      render(res[0], res[1].events || []);
    }).catch(function () {
      setOnline(false);
      if (!state.summary) render(emptySummary(state.window), []);
    });
  }

  function schedule() {
    clearInterval(state.timer);
    if (!DEMO) state.timer = setInterval(function () { if (!document.hidden) load(); }, REFRESH_MS);
  }

  function setOnline(on) {
    state.online = on;
    var st = $("status");
    st.setAttribute("data-state", on ? "on" : "off");
    $("status-text").textContent = on ? "Protected" : "Offline";
    $("offline").hidden = on;
  }

  function emptySummary(w) {
    var r = windowRange(w, Date.now());
    var tl = [];
    for (var i = 0; i < r.points; i++) tl.push({ t: r.from + i * r.bucket, requests: 0, masked: 0, values: 0, blocked: 0 });
    return { enabled: true, window: w, from: r.from, to: Date.now(), bucket_ms: r.bucket, totals: {}, timeline: tl,
      kinds: [], providers: [], clients: [], endpoints: [], block_reasons: [] };
  }

  // ---------------------------------------------------------------- render
  function render(sum, incidents) {
    state.summary = sum;
    state.incidents = incidents;
    var t = sum.totals || {};
    renderStat("stat-requests", t.requests, t.requests
      ? pct(t.masked_requests, t.requests) + " contained sensitive data" : "Waiting for AI traffic",
      sum.timeline.map(function (p) { return p.requests; }));
    renderStat("stat-values", t.values_protected, t.values_protected
      ? fmt(t.values_restored) + " restored in responses" : "Nothing sensitive yet",
      sum.timeline.map(function (p) { return p.values; }));
    renderStat("stat-blocked", t.blocked, t.blocked && sum.block_reasons.length
      ? sum.block_reasons[0].label : "Nothing blocked",
      sum.timeline.map(function (p) { return p.blocked; }));
    var top = sum.providers[0];
    renderStat("stat-dest", t.providers, t.providers
      ? plural(t.clients || 0, "app") + " · mostly " + providerName(top.key, top.label) : "No destinations yet", null);
    var chips = clear(document.querySelector("#stat-dest [data-c]"));
    sum.providers.slice(0, 4).forEach(function (p, i) {
      chips.appendChild(h("span", { class: "mono-chip", title: providerName(p.key, p.label),
        style: { background: providerColor(p.key, i) }, text: monogram(p.key, p.label) }));
    });
    renderTimeline(sum);
    renderKinds(sum);
    renderProviders(sum);
    renderClients(sum);
    renderIncidents(incidents);
  }

  function renderStat(id, value, foot, series) {
    var el = $(id);
    el.querySelector("[data-v]").textContent = fmt(value || 0);
    el.querySelector("[data-f]").textContent = foot;
    var sv = el.querySelector("[data-s]");
    if (!sv) return;
    clear(sv);
    if (!series || !series.length) return;
    var max = Math.max.apply(null, series) || 1, n = series.length;
    var pts = series.map(function (v, i) {
      return [(i / Math.max(1, n - 1)) * 120, 30 - (v / max) * 26];
    });
    var line = "M" + pts.map(function (p) { return p[0].toFixed(1) + " " + p[1].toFixed(1); }).join(" L");
    sv.appendChild(s("path", { class: "area", d: line + " L120 32 L0 32 Z" }));
    sv.appendChild(s("path", { class: "line", d: line }));
  }

  function niceMax(v) {
    // Four grid steps, each a whole "nice" number.
    var step = Math.max(1, v / 4), p = Math.pow(10, Math.floor(Math.log10(step)));
    var opts = p >= 10 ? [1, 2, 2.5, 5, 10] : [1, 2, 5, 10];
    var m = opts.find(function (x) { return x * p >= step; });
    return m * p * 4;
  }

  function tickLabel(t, w) {
    var d = new Date(t);
    if (w === "1h" || w === "24h") return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
    if (w === "7d") return d.toLocaleDateString(undefined, { weekday: "short" });
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  function renderTimeline(sum) {
    var box = clear($("timeline"));
    var W = Math.max(280, box.clientWidth || 600), H = box.clientHeight || 200;
    var padL = 34, padB = 22, padT = 6, iw = W - padL - 4, ih = H - padB - padT;
    var tl = sum.timeline || [];
    var max = niceMax(Math.max.apply(null, tl.map(function (p) { return p.requests; }).concat([0])));
    var svg = s("svg", { viewBox: "0 0 " + W + " " + H, role: "img", "aria-label": "Requests over time" });
    var grid = s("g", { class: "grid" }), axis = s("g", { class: "axis" });
    for (var g = 0; g <= 4; g++) {
      var y = padT + ih - (g / 4) * ih;
      grid.appendChild(s("line", { x1: padL, x2: W - 4, y1: y.toFixed(0) + ".5", y2: y.toFixed(0) + ".5" }));
      axis.appendChild(s("text", { x: padL - 8, y: y + 3.5, "text-anchor": "end", text: fmt((max * g) / 4) }));
    }
    svg.appendChild(grid);
    var n = Math.max(1, tl.length), slot = iw / n, bw = Math.max(2, Math.min(22, slot * 0.68));
    var every = Math.max(1, Math.ceil(n / Math.max(2, Math.floor(iw / 78))));
    var lastTick = "", lastTickX = -1e9;
    tl.forEach(function (p, i) {
      var x = padL + i * slot + (slot - bw) / 2;
      var clean = Math.max(0, p.requests - p.masked - p.blocked);
      var col = s("g", { class: "col" });
      var base = padT + ih;
      [["masked", p.masked], ["clean", clean], ["blocked", p.blocked]].forEach(function (seg) {
        if (!seg[1]) return;
        var hgt = Math.max(1.5, (seg[1] / max) * ih);
        base -= hgt;
        col.appendChild(s("rect", { class: seg[0], x: x.toFixed(1), y: base.toFixed(1), width: bw.toFixed(1), height: hgt.toFixed(1), rx: Math.min(3, bw / 3) }));
      });
      var hit = s("rect", { class: "hit", x: (padL + i * slot).toFixed(1), y: padT, width: slot.toFixed(1), height: ih });
      hit.addEventListener("mousemove", function (ev) { showTip(ev, p, sum); });
      hit.addEventListener("mouseleave", hideTip);
      col.appendChild(hit);
      svg.appendChild(col);
      // Hourly windows: evenly spaced ticks. Day windows: one tick where the day changes.
      var label = tickLabel(p.t, sum.window), tx = padL + i * slot + slot / 2;
      var daily = sum.window === "7d" || sum.window === "30d";
      if (daily ? label !== lastTick && tx - lastTickX >= 56 : i % every === 0) {
        axis.appendChild(s("text", { x: tx.toFixed(1), y: H - 6, "text-anchor": "middle", text: label }));
        lastTickX = tx;
      }
      lastTick = label;
    });
    svg.appendChild(axis);
    box.appendChild(svg);
  }

  function showTip(ev, p, sum) {
    var tip = $("tip");
    clear(tip);
    var end = p.t + (sum.bucket_ms || 60000);
    var long = sum.window === "7d" || sum.window === "30d";
    append(tip, [
      h("div", { class: "dim", text: clock(p.t, long) + " – " + clock(end, false) }),
      h("div", null, [h("b", { text: fmt(p.requests) }), " requests · ", h("b", { text: fmt(p.values) }), " values protected"]),
      p.blocked ? h("div", null, [h("b", { text: fmt(p.blocked) }), " blocked"]) : null
    ]);
    tip.hidden = false;
    var x = Math.min(ev.clientX + 14, window.innerWidth - tip.offsetWidth - 8);
    tip.style.left = x + "px";
    tip.style.top = Math.max(8, ev.clientY - tip.offsetHeight - 12) + "px";
  }
  function hideTip() { $("tip").hidden = true; }

  function barRows(el, rows, valueOf, labelOf, sub, icon, danger) {
    clear(el);
    if (!rows.length) { el.appendChild(h("div", { class: "empty", text: "Nothing yet in this window" })); return; }
    var max = Math.max.apply(null, rows.map(valueOf)) || 1;
    rows.forEach(function (r) {
      var fill = h("div", { class: "bar-fill" + (danger ? " danger" : "") });
      el.appendChild(h("div", { class: "bar-row" }, [
        h("div", { class: "bar-name", title: labelOf(r) }, [icon ? icon(r) : null, h("span", { text: labelOf(r) })]),
        h("div", { class: "bar-num" }, [h("b", { text: fmt(valueOf(r)) }), sub ? " " + sub(r) : null]),
        h("div", { class: "bar-track" }, fill)
      ]));
      requestAnimationFrame(function () { fill.style.width = Math.max(2, (100 * valueOf(r)) / max) + "%"; });
    });
  }

  function renderKinds(sum) {
    var rows = sum.kinds.slice(0, 7);
    $("kinds-hint").textContent = sum.kinds.length ? plural(sum.kinds.length, "type") : "";
    barRows($("kinds"), rows, function (r) { return r.values; },
      function (r) { return KIND_LABELS[r.key] || r.label || r.key; },
      function (r) { return "in " + fmt(r.requests); },
      function (r) { return kindIcon(r.key); });
  }

  function renderClients(sum) {
    var rows = sum.clients.slice(0, 7);
    $("clients-hint").textContent = sum.clients.length ? plural(sum.clients.length, "app") : "";
    barRows($("clients"), rows, function (r) { return r.requests; },
      function (r) { return r.label || r.key; },
      function (r) { return r.values ? "· " + fmt(r.values) + " protected" : "· clean"; },
      null);
  }

  function arc(cx, cy, r, a0, a1) {
    var large = a1 - a0 > Math.PI ? 1 : 0;
    var p0 = [cx + r * Math.sin(a0), cy - r * Math.cos(a0)], p1 = [cx + r * Math.sin(a1), cy - r * Math.cos(a1)];
    return "M" + p0[0].toFixed(2) + " " + p0[1].toFixed(2) + " A" + r + " " + r + " 0 " + large + " 1 " + p1[0].toFixed(2) + " " + p1[1].toFixed(2);
  }

  function renderProviders(sum) {
    var el = clear($("providers"));
    var rows = sum.providers.slice(0, 5);
    var rest = sum.providers.slice(5).reduce(function (a, r) { return a + r.requests; }, 0);
    if (rest) rows = rows.concat([{ key: "_other", label: "Other", requests: rest, values: 0 }]);
    var total = rows.reduce(function (a, r) { return a + r.requests; }, 0);
    $("providers-hint").textContent = sum.providers.length ? plural(sum.providers.length, "provider") : "";
    var svg = s("svg", { viewBox: "0 0 120 120", role: "img", "aria-label": "Requests by destination" });
    svg.appendChild(s("circle", { class: "track", cx: 60, cy: 60, r: 48, "stroke-width": 14 }));
    var a = 0, gap = rows.length > 1 ? 0.03 : 0;
    rows.forEach(function (r, i) {
      var frac = total ? r.requests / total : 0;
      if (frac <= 0) return;
      var a1 = a + frac * Math.PI * 2;
      if (frac >= 0.999) {
        svg.appendChild(s("circle", { cx: 60, cy: 60, r: 48, fill: "none", stroke: providerColor(r.key, i), "stroke-width": 14 }));
      } else {
        svg.appendChild(s("path", { d: arc(60, 60, 48, a + gap / 2, a1 - gap / 2), fill: "none", stroke: r.key === "_other" ? "#8a8aa3" : providerColor(r.key, i), "stroke-width": 14 }));
      }
      a = a1;
    });
    svg.appendChild(s("text", { class: "center-num", x: 60, y: 62, "text-anchor": "middle", text: fmt(total) }));
    svg.appendChild(s("text", { class: "center-lbl", x: 60, y: 77, "text-anchor": "middle", text: "requests" }));
    var legend = h("div", { class: "dlegend" });
    if (!rows.length) legend.appendChild(h("div", { class: "empty", text: "No AI traffic yet" }));
    rows.forEach(function (r, i) {
      legend.appendChild(h("div", { class: "dl-row" }, [
        h("i", { class: "sw", style: { background: r.key === "_other" ? "#8a8aa3" : providerColor(r.key, i) } }),
        h("span", { class: "dl-name", title: r.host || "", text: providerName(r.key, r.label) }),
        h("span", { class: "dl-num", text: pct(r.requests, total) })
      ]));
    });
    el.appendChild(svg);
    el.appendChild(legend);
  }

  function incidentRoute(e) {
    return h("div", { class: "route" }, [
      h("span", { class: "app", text: e.client || "Unknown" }),
      h("span", { class: "arrow", text: "→" }),
      h("span", { text: providerName(e.provider, e.provider) })
    ]);
  }
  function incidentWhat(e) {
    if (e.action === "blocked") return [h("span", { class: "tag reason", text: REASON_LABELS[e.block_reason] || "Blocked by policy" })];
    return kindTags(e.kinds, 3);
  }
  function badge(action) {
    var t = { masked: "Masked", blocked: "Blocked", clean: "Clean" }[action] || action;
    return h("span", { class: "badge " + action, text: t });
  }

  function renderIncidents(list) {
    var el = clear($("incidents"));
    if (!list.length) {
      el.appendChild(h("div", { class: "empty", text: state.online === false ? "Shadow is offline" : "No incidents yet. When Shadow masks or blocks something, it shows up here." }));
      return;
    }
    var rows = list.slice(0, 15);
    var tbody = h("tbody");
    rows.forEach(function (e) {
      tbody.appendChild(h("tr", { class: "row", onclick: function () { openSheet(e); } }, [
        h("td", { class: "when", title: new Date(e.ts).toLocaleString(), text: ago(e.ts) }),
        h("td", null, incidentRoute(e)),
        h("td", null, h("div", { class: "tags" }, incidentWhat(e))),
        h("td", null, badge(e.action)),
        h("td", null, e.restored ? h("span", { class: "restored", text: "↩ " + fmt(e.restored) }) : h("span", { class: "dim", text: "—" }))
      ]));
    });
    el.appendChild(h("table", null, [
      h("thead", null, h("tr", null, ["When", "Source → destination", "What", "Action", "Restored"].map(function (c) { return h("th", { text: c }); }))),
      tbody
    ]));
    var compact = h("div", { class: "inc-list" });
    rows.slice(0, 12).forEach(function (e) {
      compact.appendChild(h("div", { class: "inc", onclick: function () { openSheet(e); } }, [
        incidentRoute(e),
        h("div", { class: "when", text: ago(e.ts) }),
        h("div", { class: "tags" }, [badge(e.action)].concat(incidentWhat(e)))
      ]));
    });
    el.appendChild(compact);
  }

  function openSheet(e) {
    var body = clear($("sheet-body"));
    $("sheet-title").textContent = (e.action === "blocked" ? "Blocked request" : "Masked request") + " · " + clock(e.ts, true);
    var rows = [
      ["Action", badge(e.action)],
      ["App", e.client || "Unknown"],
      ["Destination", providerName(e.provider, e.provider) + (e.host ? " (" + e.host + ")" : "")],
      ["Endpoint", { chat: "Chat & completions", embeddings: "Embeddings", files: "File upload", other: "Other API call" }[e.endpoint] || e.endpoint],
      ["Route", e.mode === "gateway" ? "Gateway (/gw)" : "HTTPS proxy"],
      e.intent ? ["Intent", e.intent] : null,
      e.action === "blocked" ? ["Reason", REASON_LABELS[e.block_reason] || e.block_reason || "Policy"] : null,
      Object.keys(e.kinds || {}).length ? ["Protected", h("div", { class: "tags" }, kindTags(e.kinds, 20))] : null,
      e.detectors && e.detectors.length ? ["Detectors", e.detectors.join(", ")] : null,
      ["Restored", e.restored ? plural(e.restored, "value") + " put back in the response" : "None"],
      ["Response", (e.streamed ? "Streamed · " : "") + "HTTP " + (e.status || "—")],
      ["Size", bytes(e.bytes_in) + " sent · " + bytes(e.bytes_out) + " received"],
      ["Time", fmt(e.latency_ms) + " ms"]
    ].filter(Boolean);
    var dl = h("dl", { class: "kv" });
    rows.forEach(function (r) { dl.appendChild(h("dt", { text: r[0] })); dl.appendChild(h("dd", null, r[1])); });
    body.appendChild(dl);
    body.appendChild(h("div", { class: "never" }, [h("b", { text: "Never recorded: " }),
      "the real values, the fakes sent in their place, prompts, responses, file names and headers. This record holds counts only."]));
    $("overlay").hidden = false;
    $("sheet-close").focus();
  }
  function closeSheet() { $("overlay").hidden = true; }

  // ---------------------------------------------------------------- windows
  function windowRange(w, now) {
    var sec = Math.floor(now / 1000), spec = { "1h": [60, 60], "24h": [1800, 48], "7d": [10800, 56], "30d": [43200, 60] }[w] || [1800, 48];
    var from = sec - (sec % spec[0]) - (spec[1] - 1) * spec[0];
    return { from: from * 1000, bucket: spec[0] * 1000, points: spec[1] };
  }

  function setWindow(w) {
    state.window = w;
    try { localStorage.setItem("shadow.window", w); } catch (e) { /* ignore */ }
    Array.prototype.forEach.call(document.querySelectorAll("#windows button"), function (b) {
      b.setAttribute("aria-pressed", b.getAttribute("data-w") === w ? "true" : "false");
    });
    load();
  }

  // ---------------------------------------------------------------- demo data
  function rng(seed) {
    return function () {
      seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
      var t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function pick(r, weighted) {
    var total = weighted.reduce(function (a, w) { return a + w[1]; }, 0), x = r() * total;
    for (var i = 0; i < weighted.length; i++) { x -= weighted[i][1]; if (x <= 0) return weighted[i][0]; }
    return weighted[weighted.length - 1][0];
  }
  var DEMO_CLIENTS = [
    ["Claude Code", 30, [["anthropic", 1]]],
    ["Codex CLI", 20, [["openai", 1]]],
    ["OpenAI Python SDK", 14, [["openai", 0.7], ["azure", 0.2], ["openrouter", 0.1]]],
    ["Aider", 9, [["anthropic", 0.5], ["openai", 0.3], ["openrouter", 0.2]]],
    ["Google GenAI SDK", 7, [["gemini", 1]]],
    ["Anthropic Node SDK", 6, [["anthropic", 1]]],
    ["LangChain", 5, [["openai", 0.5], ["mistral", 0.5]]],
    ["Anthropic Python SDK", 5, [["anthropic", 1]]],
    ["curl", 4, [["openai", 0.6], ["anthropic", 0.4]]]
  ];
  var DEMO_HOSTS = {
    openai: "api.openai.com", anthropic: "api.anthropic.com", gemini: "generativelanguage.googleapis.com",
    azure: "acme-eu.openai.azure.com", openrouter: "openrouter.ai", mistral: "api.mistral.ai"
  };
  var DEMO_KINDS = [["email", 26], ["person", 22], ["secret", 12], ["hostname", 11], ["org", 8], ["term", 8],
    ["phone", 6], ["ip", 6], ["username", 5], ["address", 3], ["credit_card", 2], ["iban", 1.5], ["national_id", 1]];
  var DEMO_DETECTORS = { email: "email", person: "names", secret: "secret", hostname: "hostname", org: "dictionary",
    term: "dictionary", phone: "phone", ip: "ip", username: "local", address: "address", credit_card: "credit_card",
    iban: "iban", national_id: "national_id" };

  var demoCache = {};
  function demoEvents(now) {
    var key = Math.floor(now / 60000);
    if (demoCache.key === key) return demoCache.events;
    var r = rng(7), events = [], id = 1, start = key * 60 - 30 * 86400;
    for (var m = start; m <= key * 60; m += 60) {
      var hoursAgo = (key * 60 - m) / 3600;
      var hour = (((16 - hoursAgo) % 24) + 24) % 24; // "now" is always mid-afternoon
      var day = Math.floor((hoursAgo + 8) / 24) % 7;
      var weekend = day === 5 || day === 6;
      var busy = hour >= 9 && hour < 19 ? 1 : hour >= 7 && hour < 22 ? 0.35 : 0.05;
      var rate = (weekend ? 0.18 : 1) * busy * 1.1 * (0.75 + 0.5 * Math.sin(m / 5400) * Math.sin(m / 21000 + 1));
      var n = 0, x = r();
      var lam = Math.max(0, rate), p = Math.exp(-lam), c = p;
      while (x > c && n < 12) { n++; p *= lam / n; c += p; }
      for (var k = 0; k < n; k++) {
        var cl = pick(r, DEMO_CLIENTS.map(function (c) { return [c, c[1]]; }));
        var prov = pick(r, cl[2]);
        var roll = r(), action = roll < 0.022 ? "blocked" : roll < 0.6 ? "masked" : "clean";
        var kinds = {}, dets = {};
        if (action === "masked") {
          var nk = 1 + Math.floor(r() * r() * 4);
          for (var j = 0; j < nk; j++) {
            var kd = pick(r, DEMO_KINDS);
            kinds[kd] = (kinds[kd] || 0) + 1 + Math.floor(r() * r() * 5);
            dets[DEMO_DETECTORS[kd]] = 1;
          }
        }
        var values = Object.keys(kinds).reduce(function (a, q) { return a + kinds[q]; }, 0);
        var endpoint = pick(r, [["chat", 88], ["embeddings", 8], ["files", 4]]);
        var streamed = endpoint === "chat" && r() < 0.7;
        events.push({
          id: id++, ts: m * 1000 + Math.floor(r() * 59000), provider: prov, host: DEMO_HOSTS[prov] || prov,
          client: cl[0], mode: r() < 0.7 ? "proxy" : "gateway", method: "POST", endpoint: endpoint, action: action,
          kinds: kinds, detectors: Object.keys(dets), restored: action === "masked" && endpoint === "chat" ? Math.round(values * (0.4 + r() * 0.6)) : 0,
          intent: pick(r, [["code", 45], ["converse", 30], ["report", 15], ["act", 10]]), streamed: streamed,
          bytes_in: 800 + Math.floor(r() * 24000), bytes_out: action === "blocked" ? 160 : 400 + Math.floor(r() * 9000),
          latency_ms: action === "blocked" ? 2 + Math.floor(r() * 6) : 380 + Math.floor(r() * (streamed ? 9000 : 2400)),
          status: action === "blocked" ? 403 : 200,
          block_reason: action === "blocked" ? pick(r, [["image_blocked", 5], ["pdf_blocked", 2], ["file_too_large", 2], ["unreadable_pdf", 1]]) : undefined
        });
      }
    }
    events.sort(function (a, b) { return a.ts - b.ts; });
    events.forEach(function (e, i) { e.id = i + 1; });
    demoCache = { key: key, events: events };
    return events;
  }

  function aggregate(events, w, now) {
    var rg = windowRange(w, now), tl = [], i;
    for (i = 0; i < rg.points; i++) tl.push({ t: rg.from + i * rg.bucket, requests: 0, masked: 0, values: 0, blocked: 0 });
    var t = { requests: 0, masked_requests: 0, blocked: 0, clean: 0, values_protected: 0, values_restored: 0, bytes_in: 0, bytes_out: 0, streamed: 0, providers: 0, clients: 0, avg_latency_ms: 0 };
    var maps = { kinds: {}, providers: {}, clients: {}, endpoints: {}, block_reasons: {} }, lat = 0;
    function bump(m, k, req, val, blk) {
      var x = m[k] || (m[k] = { key: k, requests: 0, values: 0, blocked: 0 });
      x.requests += req; x.values += val; x.blocked += blk;
    }
    events.forEach(function (e) {
      if (e.ts < rg.from || e.ts > now) return;
      var v = Object.keys(e.kinds).reduce(function (a, k) { return a + e.kinds[k]; }, 0), b = e.action === "blocked" ? 1 : 0;
      t.requests++; t.masked_requests += e.action === "masked" ? 1 : 0; t.blocked += b; t.values_protected += v;
      t.values_restored += e.restored; t.bytes_in += e.bytes_in; t.bytes_out += e.bytes_out; t.streamed += e.streamed ? 1 : 0; lat += e.latency_ms;
      var p = tl[Math.floor((e.ts - rg.from) / rg.bucket)];
      if (p) { p.requests++; p.masked += e.action === "masked" ? 1 : 0; p.values += v; p.blocked += b; }
      bump(maps.providers, e.provider, 1, v, b); bump(maps.clients, e.client, 1, v, b); bump(maps.endpoints, e.endpoint, 1, v, b);
      Object.keys(e.kinds).forEach(function (k) { bump(maps.kinds, k, 1, e.kinds[k], 0); });
      if (e.block_reason) bump(maps.block_reasons, e.block_reason, 1, 0, 1);
    });
    t.clean = t.requests - t.masked_requests - t.blocked;
    t.providers = Object.keys(maps.providers).length; t.clients = Object.keys(maps.clients).length;
    t.avg_latency_ms = t.requests ? Math.round(lat / t.requests) : 0;
    function rows(m, label, byValues) {
      return Object.keys(m).map(function (k) { var x = m[k]; x.label = label(k); return x; }).sort(function (a, b) {
        return byValues ? b.values - a.values || b.requests - a.requests : b.requests - a.requests || b.values - a.values;
      });
    }
    return {
      enabled: true, window: w, from: rg.from, to: now, bucket_ms: rg.bucket, totals: t, timeline: tl,
      kinds: rows(maps.kinds, function (k) { return KIND_LABELS[k] || k; }, true),
      providers: rows(maps.providers, function (k) { return providerName(k, k); }, false),
      clients: rows(maps.clients, function (k) { return k; }, false),
      endpoints: rows(maps.endpoints, function (k) { return k; }, false),
      block_reasons: rows(maps.block_reasons, function (k) { return REASON_LABELS[k] || k; }, false),
      last_id: events.length, since: events.length ? events[0].ts : now
    };
  }

  function demoData(w) {
    var now = Date.now(), events = demoEvents(now);
    var incidents = [];
    for (var i = events.length - 1; i >= 0 && incidents.length < 60; i--) {
      if (events[i].action !== "clean" && events[i].ts <= now) incidents.push(events[i]);
    }
    return { summary: aggregate(events, w, now), incidents: incidents };
  }

  // ---------------------------------------------------------------- boot
  function boot() {
    $("demo-badge").hidden = !DEMO;
    if (DEMO) document.title = "Shadow · Activity (demo)";
    Array.prototype.forEach.call(document.querySelectorAll("#windows button"), function (b) {
      b.addEventListener("click", function () { setWindow(b.getAttribute("data-w")); });
    });
    $("sheet-close").addEventListener("click", closeSheet);
    $("overlay").addEventListener("click", function (e) { if (e.target === $("overlay")) closeSheet(); });
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape") closeSheet();
      var idx = ["1", "2", "3", "4"].indexOf(e.key);
      if (idx >= 0 && !e.metaKey && !e.ctrlKey && !e.altKey) setWindow(WINDOWS[idx]);
    });
    document.addEventListener("visibilitychange", function () { if (!document.hidden) load(); });
    var resizeT, lastW = 0;
    var redraw = function () {
      var w = $("timeline").clientWidth;
      if (w === lastW || !state.summary) return;
      lastW = w;
      clearTimeout(resizeT);
      resizeT = setTimeout(function () { renderTimeline(state.summary); }, 60);
    };
    if (window.ResizeObserver) new ResizeObserver(redraw).observe($("timeline"));
    window.addEventListener("resize", redraw);
    setWindow(state.window);
    schedule();
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", boot);
  else boot();
})();
