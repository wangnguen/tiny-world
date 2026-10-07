// Dựng trang giới thiệu (website/) từ asset, code và file chữ của app, để trang luôn khớp với app:
// - website/index.html (tiếng Việt), website/en/index.html (tiếng Anh): khung website/i18n/page.html điền chữ
//   trong website/i18n/vi.json, en.json; tên, tính cách nhân vật lấy ở file chữ của app (packages/core/src/i18n)
// - website/pets/<pack>/<animation>.webp: mỗi animation một dải frame ngang (cắt từ atlas nếu pack dùng atlas)
// - website/pets/pets.json: tên, animation, điểm chân, thân, miệng, tầm tay lúc leo của từng con
// - website/js/gen/*.js: module dùng chung với app (thời tiết quanh pet, câu pet nói, âm lịch, hình thời tiết),
//   chỉ bỏ phần khai báo kiểu TypeScript
// - website/img/, website/favicon.ico: logo, favicon, ảnh xem trước khi chia sẻ link
// Chạy lại sau khi sửa chữ, khung trang, sprite hay các module trên: pnpm prepare:website
// Link đầy đủ cho máy tìm kiếm, mạng xã hội ghi theo SITE_URL, mặc định là trang thật https://tinyworld-1bw.pages.dev;
// đổi tên miền thì đặt SITE_URL, đặt rỗng thì chỉ dùng link tương đối.
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import sharp from "sharp";
import ts from "typescript";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const PACKS = join(ROOT, "assets/sprites");
const SITE = join(ROOT, "website");
const SITE_URL = (process.env.SITE_URL ?? "https://tinyworld-1bw.pages.dev").replace(/\/+$/, "");
const REPO = "https://github.com/wangnguen/tiny-world";

/** Các trang: tiếng Việt ở gốc, tiếng Anh ở /en/. */
const LANGS = {
  vi: { path: "/", out: "index.html", locale: "vi_VN" },
  en: { path: "/en/", out: "en/index.html", locale: "en_US" },
};

/**
 * Module của app dùng lại trên trang. Chỉ được import kiểu (bị bỏ khi dịch), trừ `@tinyworld/core` đổi sang
 * core.js của trang (`fill` và chữ của trang đang mở).
 */
const SHARED = {
  "weather.js": "desktop/src/overlay/weather.ts",
  "lines.js": "packages/sim/src/lines.ts",
  "lunar.js": "packages/sim/src/lunar.ts",
  "skyIcons.js": "desktop/src/skyIcons.ts",
};

/** core.js của trang: thay `@tinyworld/core` cho lines.js, chữ của app nằm sẵn trong trang (#tw-app-text). */
const CORE_SHIM = `// Sinh bởi scripts/prepare-website.mjs, không sửa tay. Thay @tinyworld/core cho lines.js trên trang giới
// thiệu: chữ của app (câu pet nói, tên thứ, tháng) theo ngôn ngữ của trang nằm sẵn trong #tw-app-text.
const TEXT = JSON.parse(document.getElementById("tw-app-text")?.textContent ?? "{}");

export function messages() {
  return TEXT;
}

export function fill(template, params) {
  return template.replace(/\\{(\\w+)\\}/g, (match, name) => (name in params ? String(params[name]) : match));
}
`;

const SKIES = ["sunny", "clear", "cloudy", "rain", "storm", "snow", "fog", "petals"];
/** Nút Nóng, Lạnh trong demo thời tiết (°C), như mục Xem thử trong Cài đặt. */
const HOT = 36;
const COLD = 8;

/** Ảnh xem trước khi chia sẻ link: các con đứng trên đồi, theo thứ tự này. */
const OG_PETS = ["b-kitsu", "c-lumi", "a-momo", "b-bong", "c-boggo", "c-tan"];

const sitePath = (...parts) => join(SITE, ...parts);
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));

/** Khung bao phần có hình (alpha > 0) của một frame, pixel của frame; null nếu frame trống. */
function bounds(data, info, left, top, width, height) {
  let x0 = width;
  let y0 = height;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (data[((top + y) * info.width + left + x) * 4 + 3] === 0) continue;
      x0 = Math.min(x0, x);
      x1 = Math.max(x1, x);
      y0 = Math.min(y0, y);
      y1 = Math.max(y1, y);
    }
  }
  return x1 < 0 ? null : { x: x0, y: y0, width: x1 - x0 + 1, height: y1 - y0 + 1 };
}

async function preparePets() {
  rmSync(sitePath("pets"), { recursive: true, force: true });
  const pets = [];
  for (const id of readdirSync(PACKS).sort()) {
    const pack = readJson(join(PACKS, id, "pet.json"));
    const { frameWidth: fw, frameHeight: fh, anchor } = pack;
    mkdirSync(sitePath("pets", id), { recursive: true });
    const animations = {};
    const raws = new Map();
    for (const [name, a] of Object.entries(pack.animations)) {
      const file = join(PACKS, id, a.image);
      if (!raws.has(file)) raws.set(file, await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true }));
      const top = (a.row ?? 0) * fh;
      const out = sitePath("pets", id, `${name}.webp`);
      const { info } = raws.get(file);
      if (top === 0 && info.width === fw * a.frames && info.height === fh) copyFileSync(file, out);
      else await sharp(file).extract({ left: 0, top, width: fw * a.frames, height: fh }).webp({ lossless: true }).toFile(out);
      animations[name] = { frames: a.frames, fps: a.fps, loop: a.loop };
    }

    // Thân: phần có hình của frame đầu `idle` (như PetView.body của app). Tầm tay lúc leo: mép tay xa nhất.
    const idle = pack.animations.idle;
    const { data, info } = raws.get(join(PACKS, id, idle.image));
    const idleTop = (idle.row ?? 0) * fh;
    const body = bounds(data, info, 0, idleTop, fw, fh);
    const climb = pack.animations.climb;
    const climbRaw = raws.get(join(PACKS, id, climb.image));
    let reach = fw * 0.1;
    for (let f = 0; f < climb.frames; f++) {
      const box = bounds(climbRaw.data, climbRaw.info, f * fw, (climb.row ?? 0) * fh, fw, fh);
      if (box) reach = Math.max(reach, box.x + box.width - anchor.x);
    }
    // Miệng: mép mặt phía trước ở hàng `mouthY`; pack không ghi thì lấy hàng ở 45% chiều cao thân.
    const mouthY = pack.mouthY ?? Math.round(body.y + body.height * 0.45);
    let mouthX = anchor.x;
    for (let x = fw - 1; x >= 0; x--) {
      if (data[((idleTop + mouthY) * info.width + x) * 4 + 3] > 0) {
        mouthX = x;
        break;
      }
    }

    pets.push({
      id,
      name: pack.name.split(" — ")[0],
      frame: { width: fw, height: fh },
      anchor,
      body,
      mouth: { x: mouthX, y: mouthY },
      reach,
      animations,
    });
  }
  writeFileSync(sitePath("pets", "pets.json"), `${JSON.stringify(pets, null, 1)}\n`);
  return pets;
}

async function prepareModules() {
  rmSync(sitePath("js/gen"), { recursive: true, force: true });
  mkdirSync(sitePath("js/gen"), { recursive: true });
  for (const [out, from] of Object.entries(SHARED)) {
    const source = readFileSync(join(ROOT, from), "utf8");
    let { outputText } = ts.transpileModule(source, {
      compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
      fileName: from,
    });
    outputText = outputText.replace(/from "@tinyworld\/core"/g, 'from "./core.js"');
    const imports = [...outputText.matchAll(/^\s*import\s.*\sfrom\s+"([^"]+)"/gm)].map((m) => m[1]);
    const unknown = imports.filter((path) => path !== "./core.js");
    if (unknown.length) throw new Error(`${from} còn import ${unknown.join(", ")}, trang web không chạy được module này.`);
    const header = `// Sinh từ ${from} bằng scripts/prepare-website.mjs, không sửa tay.\n`;
    writeFileSync(sitePath("js/gen", out), header + outputText.replace(/\r\n/g, "\n"));
  }
  writeFileSync(sitePath("js/gen/core.js"), CORE_SHIM);
  return import(pathToFileURL(sitePath("js/gen/skyIcons.js")).href);
}

async function prepareImages(pets) {
  mkdirSync(sitePath("img"), { recursive: true });
  const icon = join(ROOT, "assets/icon.png");
  await sharp(icon).resize(256, 256).png().toFile(sitePath("img/logo.png"));
  // Icon iOS: nền đêm, không trong suốt (trong suốt thì iOS tô đen).
  await sharp({ create: { width: 180, height: 180, channels: 4, background: "#12203f" } })
    .composite([{ input: await sharp(icon).resize(150, 150).toBuffer(), left: 15, top: 15 }])
    .png()
    .toFile(sitePath("img/apple-touch-icon.png"));
  copyFileSync(join(ROOT, "desktop/src-tauri/icons/32x32.png"), sitePath("img/favicon-32.png"));
  copyFileSync(join(ROOT, "desktop/src-tauri/icons/icon.ico"), sitePath("favicon.ico"));

  // Ảnh xem trước khi chia sẻ link (1200×630): trời đêm, đồi, vài con đứng trên đồi, logo và tên app. Chữ trên
  // ảnh là tiếng Anh, dùng chung cho cả hai trang.
  const width = 1200;
  const height = 630;
  const stars = Array.from({ length: 70 }, (_, i) => {
    const x = (i * 397 + 83) % width;
    const y = (i * 151 + 37) % 360;
    // Không chấm sao vào chỗ chữ và logo.
    if (x > 250 && x < 950 && y > 30 && y < 320) return "";
    const r = 0.8 + ((i * 7) % 5) * 0.35;
    return `<circle cx="${x}" cy="${y}" r="${r}" fill="#eef2ff" opacity="${0.35 + ((i * 13) % 6) / 10}"/>`;
  }).join("");
  // Mép đồi: thấp ở giữa, cao dần hai bên; chân các con đặt đúng lên đường này.
  const hillY = (x) => 482 + ((x - width / 2) / (width / 2)) ** 2 * 34;
  const ridge = Array.from({ length: 41 }, (_, i) => `${(i * width) / 40} ${hillY((i * width) / 40).toFixed(1)}`);
  const backdrop = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
    <defs>
      <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#12203f"/><stop offset="1" stop-color="#0b1426"/></linearGradient>
      <radialGradient id="glow" cx="0.8" cy="0.05" r="0.7"><stop offset="0" stop-color="#2d58a8" stop-opacity="0.75"/><stop offset="1" stop-color="#2d58a8" stop-opacity="0"/></radialGradient>
      <linearGradient id="hill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#315c66"/><stop offset="0.5" stop-color="#1f3a4f"/><stop offset="1" stop-color="#13213a"/></linearGradient>
    </defs>
    <rect width="${width}" height="${height}" fill="url(#sky)"/>
    <rect width="${width}" height="${height}" fill="url(#glow)"/>
    ${stars}
    <path d="M${ridge.join("L")}V${height}H0Z" fill="url(#hill)"/>
    <path d="M${ridge.join("L")}" fill="none" stroke="#86c8a4" stroke-opacity="0.75" stroke-width="3"/>
    <text x="600" y="232" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="104" font-weight="700" fill="#eef2fd">TinyWorld</text>
    <text x="600" y="296" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="36" fill="#b9c8ea">Tiny pets living on your Windows desktop</text>
  </svg>`;
  const layers = [{ input: await sharp(icon).resize(120, 120).toBuffer(), left: 540, top: 40 }];
  const size = 150;
  const step = 168;
  const first = (width - step * (OG_PETS.length - 1) - size) / 2;
  for (const [i, id] of OG_PETS.entries()) {
    const pet = pets.find((p) => p.id === id);
    if (!pet) continue;
    const frame = await sharp(sitePath("pets", id, "idle.webp"))
      .extract({ left: 0, top: 0, width: pet.frame.width, height: pet.frame.height })
      .resize(size, size)
      .toBuffer();
    // Điểm chân (anchor) chạm mép đồi.
    const x = Math.round(first + i * step);
    const ground = hillY(x + (pet.anchor.x / pet.frame.width) * size) + 2;
    layers.push({ input: frame, left: x, top: Math.round(ground - (pet.anchor.y / pet.frame.height) * size) });
  }
  await sharp(Buffer.from(backdrop)).composite(layers).png().toFile(sitePath("img/og.png"));
}

const escape = (text) =>
  String(text).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

/** JSON để nằm trong thẻ <script>: không để lọt "</script>". */
const scriptJson = (value) => JSON.stringify(value).replace(/</g, "\\u003c");

const fill = (template, params) => template.replace(/\{(\w+)\}/g, (match, name) => (name in params ? String(params[name]) : match));

const icon = (name) => `<svg class="icon"><use href="#i-${name}" /></svg>`;

/** Mục và độ dài mảng của một file chữ, để so hai ngôn ngữ. */
function shape(value, path = "") {
  if (Array.isArray(value)) return [`${path}[${value.length}]`, ...value.flatMap((v, i) => shape(v, `${path}[${i}]`))];
  if (value && typeof value === "object") {
    return Object.entries(value).flatMap(([key, v]) => [`${path}.${key}`, ...shape(v, `${path}.${key}`)]);
  }
  return [];
}

function preparePages(pets, { skyIconSvg }) {
  const template = readFileSync(sitePath("i18n/page.html"), "utf8").replace(/\r\n/g, "\n");
  const site = Object.fromEntries(Object.keys(LANGS).map((lang) => [lang, readJson(sitePath(`i18n/${lang}.json`))]));
  const app = Object.fromEntries(
    Object.keys(LANGS).map((lang) => [lang, readJson(join(ROOT, `packages/core/src/i18n/${lang}.json`))]),
  );
  const [vi, en] = [shape(site.vi).sort().join("\n"), shape(site.en).sort().join("\n")];
  if (vi !== en) throw new Error("website/i18n/vi.json và en.json không có đúng các mục giống nhau.");
  const url = (path) => `${SITE_URL}${path}`;

  for (const [lang, page] of Object.entries(LANGS)) {
    const t = site[lang];
    const words = app[lang];
    const [otherLang, other] = Object.entries(LANGS).find(([code]) => code !== lang);
    const links = [
      ...Object.entries(LANGS).map(([code, p]) => `<link rel="alternate" hreflang="${code}" href="${url(p.path)}" />`),
      `<link rel="alternate" hreflang="x-default" href="${url(LANGS.vi.path)}" />`,
      `<meta property="og:image" content="${url("/img/og.png")}" />`,
      ...(SITE_URL
        ? [`<link rel="canonical" href="${url(page.path)}" />`, `<meta property="og:url" content="${url(page.path)}" />`]
        : []),
    ].join("\n    ");
    const capital = (text) => text.charAt(0).toUpperCase() + text.slice(1);
    const raw = {
      lang,
      locale: page.locale,
      links,
      otherLang,
      otherHref: other.path,
      suggestText: escape(site[otherLang].nav.suggest),
      suggestAction: escape(site[otherLang].nav.suggestAction),
      notes: t.desk.notes.map((line, i) => (i === 0 ? `<strong>${escape(line)}</strong>` : `<p>${escape(line)}</p>`)).join(""),
      features: t.features.items
        .map(
          (item) =>
            `<li class="feature feature--${item.icon}"><span class="feature__icon">${icon(item.icon)}</span><h3>${escape(item.title)}</h3><p>${escape(item.text)}</p></li>`,
        )
        .join(""),
      skyChips: SKIES.map(
        (sky) =>
          `<button type="button" class="chip" data-sky="${sky}" aria-pressed="false"><span class="chip__icon">${skyIconSvg(sky)}</span>${escape(t.weather.skies[sky])}</button>`,
      ).join(""),
      warmthChips: [
        ["hot", HOT],
        ["cold", COLD],
      ]
        .map(
          ([warmth, temperature]) =>
            `<button type="button" class="chip chip--${warmth}" data-warmth="${warmth}" data-temperature="${temperature}" aria-pressed="false">${escape(fill(t.weather[warmth], { temperature }))}</button>`,
        )
        .join(""),
      roster: pets
        .map((pet) => {
          const about = words.personas[pet.id] ?? words.prompt.defaultAbout;
          return `<li><button type="button" class="pet-card" data-pet="${pet.id}" aria-pressed="false" aria-label="${escape(fill(t.pets.drop, { name: pet.name }))}"><span class="pet-card__stage"><span class="sprite sprite--card" style="background-image:url(/pets/${pet.id}/idle.webp)"></span></span><span class="pet-card__name">${escape(pet.name)}</span><span class="pet-card__about">${escape(capital(about))}</span><span class="pet-card__badge">${escape(t.pets.onScreen)}</span></button></li>`;
        })
        .join(""),
      tips: t.tips.items
        .map((item) => {
          const join = item.then ? `<span class="tip__join">→</span>` : `<span class="tip__join">+</span>`;
          return `<div class="tip"><dt>${item.keys.map((key) => `<kbd>${escape(key)}</kbd>`).join(join)}</dt><dd>${escape(item.text)}</dd></div>`;
        })
        .join(""),
      calm: t.calm.items
        .map(
          (item) =>
            `<li class="calm__item"><span class="calm__icon">${icon(item.icon)}</span><div><h3>${escape(item.title)}</h3><p>${escape(item.text)}</p></div></li>`,
        )
        .join(""),
      steps: t.download.steps.map((step) => `<li>${escape(step)}</li>`).join(""),
      releases: `${REPO}/releases/latest`,
      releasesAll: `${REPO}/releases`,
      repo: REPO,
      runtime: scriptJson({
        lang,
        desk: t.desk,
        weather: { skies: t.weather.skies },
        pets: { onScreen: t.pets.onScreen },
        download: { meta: t.download.meta },
      }),
      appText: scriptJson({ pet: words.pet, calendar: words.calendar, occasions: words.occasions }),
    };
    const lookup = (object, path) => path.split(".").reduce((node, key) => node?.[key], object);
    const html = template
      .replace(/<!-- Khung trang chung[\s\S]*?-->/, "<!-- Sinh từ website/i18n/page.html bằng scripts/prepare-website.mjs, không sửa tay. -->")
      .replace(/\{\{(t|raw)\.([\w.]+)\}\}/g, (match, kind, path) => {
        const value = kind === "t" ? lookup(t, path) : raw[path];
        if (typeof value !== "string") throw new Error(`${lang}: không có ${kind}.${path}`);
        return kind === "t" ? escape(value) : value;
      });
    mkdirSync(dirname(sitePath(page.out)), { recursive: true });
    writeFileSync(sitePath(page.out), html);
  }
}

const pets = await preparePets();
const shared = await prepareModules();
await prepareImages(pets);
preparePages(pets, shared);
console.log(`Đã dựng website/: ${Object.keys(LANGS).length} trang, ${pets.length} nhân vật, ${Object.keys(SHARED).length} module.`);
