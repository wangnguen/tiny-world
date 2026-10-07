// Sinh từ packages/sim/src/lines.ts bằng scripts/prepare-website.mjs, không sửa tay.
/**
 * Mọi câu pet nói (speech bubble), theo ngôn ngữ đang dùng. Câu chữ nằm ở mục `pet` (và `calendar`,
 * `occasions`) trong `packages/core/src/i18n/vi.json`, `en.json`: giọng thân mật, dùng ":)))" thay emoji.
 * Mỗi cửa sổ (overlay, Cài đặt) gọi `setLang` khi đọc cài đặt và mỗi lần ngôn ngữ đổi.
 */
import { fill, messages } from "./core.js";
let lang = "vi";
/** Đổi ngôn ngữ của mọi câu pet nói từ giờ trở đi. */
export function setLang(next) {
    lang = next;
}
export function currentLang() {
    return lang;
}
const text = () => messages(lang);
/** Câu chào khi gặp nhau (chọn ngẫu nhiên một câu; hai ngôn ngữ cùng số câu nên cùng seed ra cùng lượt). */
export function greetings() {
    return text().pet.greetings;
}
const degrees = (temperature) => `${Math.round(temperature)}°C`;
/** Câu khi trời vừa đổi, kèm nhiệt độ nếu biết, ví dụ "Mưa rồi, 24°C :(". Nắng mà nóng thì kêu nóng. */
export function skyLine(sky, temperature, warmth) {
    const pet = text().pet;
    const [line, mood] = sky === "sunny" && warmth === "hot" ? pet.sunnyHot : pet.sky[sky];
    if (temperature === null)
        return fill(pet.skyAlone, { text: line, mood });
    return fill(pet.skyWithTemperature, { text: line, temperature: degrees(temperature), mood });
}
/** Câu khi trời vừa chuyển nóng hay lạnh (thời tiết không đổi), ví dụ "Nóng quá, 36°C :(((". */
export function warmthLine(warmth, temperature) {
    return fill(text().pet[warmth], { temperature: degrees(temperature) });
}
/** Không lấy được thời tiết: mất mạng, hoặc máy chủ thời tiết lỗi. */
export const noNetwork = () => text().pet.noNetwork;
export const noWeather = () => text().pet.noWeather;
/** Con ma bay qua lúc 2 giờ sáng: con đầu tiên bị giật mình nói câu này. */
export const ghostLine = () => text().pet.ghost;
/** Nhắc nghỉ sau `minutes` phút ngồi liền. */
export const breakLine = (minutes) => fill(text().pet.break, { minutes });
/** Nhắc uống nước sau một lúc ngồi máy. */
export const waterLine = () => text().pet.water;
/** Nhắc đi ngủ lúc khuya, `time` ví dụ "23:40". */
export const bedtimeLine = (time) => fill(text().pet.bedtime, { time });
/** Bấm Ctrl+S dồn dập. */
export const saveSpamLine = () => text().pet.saveSpam;
/** Click chuột phải vào pet mà chưa bật chat. */
export const chatOffLine = () => text().pet.chatOff;
/** GitHub có bản mới (update.rs): một con báo, mỗi bản một lần. */
export const updateLine = (version) => fill(text().pet.update, { version });
/** Thời tiết khi bấm đúp vào pet, ví dụ "mưa" (trời nắng mùa xuân thì là `petals`, vẫn là "trời nắng"). */
export const skyName = (sky) => text().pet.skyNames[sky];
/**
 * Tên dịp và câu nói của dịp: dịp có sẵn chưa sửa thì theo ngôn ngữ `inLang` (mặc định ngôn ngữ đang dùng),
 * còn lại như người dùng ghi.
 */
export function occasionText(occasion, inLang = lang) {
    const presets = messages(inLang).occasions;
    const preset = occasion.preset ? presets[occasion.preset] : undefined;
    return preset ?? { name: occasion.name, message: occasion.message };
}
const pad = (n) => String(n).padStart(2, "0");
/** Một dòng thời tiết, ví dụ "Hà Nội: mưa, 27°C"; lấy không được thì là câu báo lỗi (`problem`). */
export function weatherLine(weather) {
    const pet = text().pet;
    if (weather.sky && weather.temperature !== undefined) {
        return fill(pet.place, { place: weather.place, sky: pet.skyNames[weather.sky], temperature: degrees(weather.temperature) });
    }
    return weather.problem ?? `${weather.place}: ${pet.weatherUnknown}`;
}
/**
 * Câu khi bấm đúp vào pet: giờ, thứ, ngày dương lịch, ngày âm lịch, và thời tiết nếu đã chọn thành phố.
 * Ví dụ "15:42 · Thứ Tư 01/10", "Âm lịch 21/8", "Hà Nội: mưa, 27°C", mỗi phần một dòng.
 */
export function nowText(date, lunar, weather) {
    const { pet, calendar } = text();
    const lines = [
        fill(pet.clock, {
            hh: pad(date.getHours()),
            min: pad(date.getMinutes()),
            weekday: calendar.weekdays[date.getDay()],
            dd: pad(date.getDate()),
            mm: pad(date.getMonth() + 1),
            day: date.getDate(),
            shortMonth: calendar.shortMonths[date.getMonth()],
        }),
        fill(pet.lunar, { day: lunar.day, month: lunar.month, leap: lunar.leap ? pet.leap : "" }),
    ];
    if (weather)
        lines.push(weatherLine(weather));
    return lines.join("\n");
}
