// Trang giới thiệu TinyWorld: trời sao ở đầu trang, màn hình thu nhỏ có pet (desk.js), demo thời tiết
// (demo.js), bảng nhân vật, nút tải bản mới nhất trên GitHub, gợi ý đổi ngôn ngữ.
import { Desk } from "./desk.js";
import { WeatherDemo } from "./demo.js";
import { petUrl } from "./sprite.js";

const LATEST = "https://api.github.com/repos/wangnguen/tiny-world/releases/latest";
/** Ba con đầu tiên trên màn hình thu nhỏ; con đầu cũng là con trong demo thời tiết. */
const START_PETS = ["a-momo", "b-kitsu", "b-bong"];
const LANG_KEY = "tinyworld.lang";

const text = JSON.parse(document.getElementById("tw-text")?.textContent ?? "{}");
const lang = document.documentElement.lang;
const fill = (template, params) => template.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));

function storage(action) {
  try {
    return action(window.localStorage);
  } catch {
    return null;
  }
}

/** Sao lấp lánh trên trời ở đầu trang (vị trí cố định theo số thứ tự, không đổi mỗi lần tải). */
function drawStars() {
  for (const sky of document.querySelectorAll("[data-stars]")) {
    const count = window.innerWidth < 720 ? 36 : 70;
    for (let i = 0; i < count; i++) {
      const star = document.createElement("span");
      const gold = i % 17 === 0;
      const size = gold ? 3 : 1 + ((i * 7) % 3) * 0.6;
      star.className = gold ? "star star--gold" : "star";
      Object.assign(star.style, {
        left: `${(i * 137.5) % 100}%`,
        top: `${(i * 61.8) % 100}%`,
        width: `${size}px`,
        height: `${size}px`,
        opacity: String(0.35 + ((i * 13) % 6) / 10),
        animationDelay: `${-((i * 0.77) % 4)}s`,
      });
      sky.append(star);
    }
  }
}

/** Trình duyệt dùng ngôn ngữ khác trang này, chưa chọn lần nào: gợi ý trang ngôn ngữ kia. */
function suggestLanguage() {
  const bar = document.querySelector("[data-suggest]");
  for (const link of document.querySelectorAll("[data-lang-link]")) {
    link.addEventListener("click", () => storage((s) => s.setItem(LANG_KEY, link.dataset.langLink)));
  }
  if (!bar || storage((s) => s.getItem(LANG_KEY))) return;
  const wantsVietnamese = (navigator.languages ?? [navigator.language]).some((l) => l.toLowerCase().startsWith("vi"));
  if (wantsVietnamese === (lang === "vi")) return;
  bar.hidden = false;
  bar.querySelector("[data-suggest-close]")?.addEventListener("click", () => {
    bar.hidden = true;
    storage((s) => s.setItem(LANG_KEY, lang));
  });
}

/** Nút tải: link thẳng tới bộ cài, bản chạy thẳng của bản mới nhất trên GitHub; lỗi thì để link trang Releases. */
async function linkDownloads() {
  const meta = document.querySelector("[data-download-meta]");
  try {
    const response = await fetch(LATEST, { headers: { Accept: "application/vnd.github+json" } });
    if (!response.ok) return;
    const release = await response.json();
    const assets = release.assets ?? [];
    const setup = assets.find((a) => a.name.endsWith("-setup.exe"));
    const portable = assets.find((a) => a.name.endsWith("-portable.exe"));
    for (const [kind, asset] of [
      ["setup", setup],
      ["portable", portable],
    ]) {
      if (!asset) continue;
      for (const link of document.querySelectorAll(`[data-download="${kind}"]`)) link.href = asset.browser_download_url;
    }
    if (setup && meta) {
      const megabytes = (setup.size / 1024 / 1024).toLocaleString(lang === "vi" ? "vi-VN" : "en-US", {
        maximumFractionDigits: 1,
      });
      meta.textContent = fill(text.download.meta, {
        version: String(release.tag_name ?? "").replace(/^v/, ""),
        size: `${megabytes} MB`,
      });
    }
  } catch {
    // Không hỏi được GitHub (mất mạng, hết lượt hỏi): nút vẫn dẫn tới trang Releases.
  }
}

/** Bảng nhân vật: rê chuột thì con đó đi, bấm thì thả xuống màn hình ở đầu trang và vào demo thời tiết. */
function setUpRoster(desk, demo, defs) {
  const cards = [...document.querySelectorAll("[data-roster] [data-pet]")];
  const byId = new Map(defs.map((def) => [def.id, def]));
  for (const card of cards) {
    const id = card.dataset.pet;
    const sprite = card.querySelector(".sprite");
    const walk = () => (sprite.style.backgroundImage = `url(${petUrl(id, "walk")})`);
    const rest = () => (sprite.style.backgroundImage = `url(${petUrl(id, "idle")})`);
    card.addEventListener("pointerenter", walk);
    card.addEventListener("pointerleave", rest);
    card.addEventListener("focus", walk);
    card.addEventListener("blur", rest);
    card.addEventListener("click", () => {
      desk.drop(id);
      const def = byId.get(id);
      if (def) demo?.setPet(def);
      const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
      document.getElementById("desk")?.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "center" });
    });
  }
  desk.onChange = (ids) => {
    for (const card of cards) card.setAttribute("aria-pressed", String(ids.includes(card.dataset.pet)));
  };
}

async function start() {
  drawStars();
  suggestLanguage();
  void linkDownloads();
  const defs = await fetch("/pets/pets.json").then((r) => r.json());
  const deskRoot = document.querySelector("[data-desk]");
  if (!deskRoot) return;
  const desk = new Desk(deskRoot, defs, text);
  const demoRoot = document.querySelector("[data-demo]");
  const first = defs.find((def) => def.id === START_PETS[0]) ?? defs[0];
  const demo = demoRoot ? new WeatherDemo(demoRoot, first) : null;
  setUpRoster(desk, demo, defs);
  desk.populate(START_PETS.filter((id) => defs.some((def) => def.id === id)));
}

start().catch((error) => console.error("TinyWorld:", error));
