# Trang giới thiệu TinyWorld

Trang tĩnh (HTML, CSS, JS thuần, không cần build) giới thiệu app, có tiếng Việt (`/`) và tiếng Anh (`/en/`).
Ở đầu trang có một màn hình thu nhỏ với pet thật của app: bấm, kéo thả, ném, bấm đúp xem giờ, kéo hay đóng cửa
sổ. Dưới đó là demo thời tiết, bảng 22 nhân vật và nút tải bản mới nhất (lấy từ GitHub Releases lúc mở trang).

## Cấu trúc

| Đường dẫn | Là gì | Sửa tay? |
|---|---|---|
| `i18n/vi.json`, `i18n/en.json` | Chữ của trang, hai file phải có đúng các mục như nhau | Có |
| `i18n/page.html` | Khung trang chung cho mọi ngôn ngữ (`{{t.mục}}` là chữ) | Có |
| `style.css`, `js/main.js`, `js/desk.js`, `js/demo.js`, `js/sprite.js` | Giao diện và hành vi | Có |
| `_headers` | Header HTTP trên Cloudflare (cache ảnh) | Có |
| `index.html`, `en/index.html` | Trang đã điền chữ | Không, sinh ra |
| `pets/`, `img/`, `favicon.ico`, `js/gen/` | Sprite, ảnh, và code dùng chung với app (thời tiết, câu pet nói, âm lịch) | Không, sinh ra |

Tên và tính cách nhân vật, câu pet nói lấy từ file chữ của app (`packages/core/src/i18n/`), nên trang luôn khớp
với app.

## Sửa trang

1. Sửa chữ trong `i18n/vi.json` và `i18n/en.json`, hoặc sửa bố cục trong `i18n/page.html`.
2. Dựng lại (cũng chạy lại khi đổi sprite, file chữ của app, hay `weather.ts`, `lines.ts` của app):

   ```bash
   pnpm prepare:website
   ```

3. Xem thử bằng một máy chủ tĩnh bất kỳ, mở ở gốc thư mục `website` (mở thẳng file `index.html` thì JS không
   chạy vì trình duyệt chặn module và `fetch` từ `file://`):

   ```bash
   npx wrangler pages dev website
   ```

Có tên miền thì dựng với `SITE_URL` để trang ghi link đầy đủ (canonical, `og:url`, ảnh xem trước khi chia sẻ):

```bash
SITE_URL=https://tinyworld.pages.dev pnpm prepare:website
```

Workflow Build của app bỏ qua các push chỉ sửa `website/`, nên sửa trang không build lại `.exe`.
