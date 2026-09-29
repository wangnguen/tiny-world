import { describe, expect, it } from "vitest";
import {
  frameIndex,
  frameRects,
  parseSpriteManifest,
  type AnimationSpec,
} from "./sprite";

const minimal = {
  frameWidth: 32,
  frameHeight: 32,
  animations: { idle: { image: "Idle.png", fps: 8 } },
};

function spec(overrides: Partial<AnimationSpec>): AnimationSpec {
  return { image: "a.png", frames: undefined, fps: 8, start: 0, row: 0, loop: true, ...overrides };
}

describe("parseSpriteManifest", () => {
  it("điền giá trị mặc định", () => {
    const manifest = parseSpriteManifest(minimal);
    expect(manifest).toMatchObject({
      name: "pet",
      scale: 2,
      pixelArt: true,
      facing: "right",
      anchor: { x: 16, y: 32 },
    });
    expect(manifest.animations.idle).toEqual(spec({ image: "Idle.png" }));
  });

  it("chuẩn hoá đường dẫn ảnh", () => {
    const manifest = parseSpriteManifest({
      ...minimal,
      animations: { idle: { image: ".\\sheets\\Idle.png", fps: 8 } },
    });
    expect(manifest.animations.idle.image).toBe("sheets/Idle.png");
  });

  it("bắt buộc có idle", () => {
    expect(() =>
      parseSpriteManifest({ ...minimal, animations: { walk: { image: "Walk.png", fps: 8 } } }),
    ).toThrow("animations.idle");
  });

  it("báo tên animation sai (ví dụ viết hoa)", () => {
    expect(() =>
      parseSpriteManifest({ ...minimal, animations: { Idle: { image: "Idle.png", fps: 8 } } }),
    ).toThrow("animations.Idle");
  });

  it("chặn đường dẫn ra ngoài thư mục pack", () => {
    for (const image of ["../x.png", "/x.png", "C:/x.png", "a/../../x.png"]) {
      expect(() =>
        parseSpriteManifest({ ...minimal, animations: { idle: { image, fps: 8 } } }),
      ).toThrow("animations.idle.image");
    }
  });

  it("chặn số ngoài khoảng", () => {
    expect(() => parseSpriteManifest({ ...minimal, frameWidth: 0 })).toThrow("frameWidth");
    expect(() => parseSpriteManifest({ ...minimal, frameWidth: 1.5 })).toThrow("frameWidth");
    expect(() =>
      parseSpriteManifest({ ...minimal, anchor: { x: 16, y: 40 } }),
    ).toThrow("anchor.y");
  });
});

describe("frameRects", () => {
  it("ảnh dải ngang, tự lấy hết frame", () => {
    const rects = frameRects(spec({}), 32, 32, 128, 32);
    expect(rects.map((r) => r.x)).toEqual([0, 32, 64, 96]);
    expect(rects.every((r) => r.y === 0 && r.width === 32 && r.height === 32)).toBe(true);
  });

  it("sheet nhiều hàng: bắt đầu ở hàng row, tràn sang hàng dưới", () => {
    const rects = frameRects(spec({ row: 1, start: 2, frames: 3 }), 16, 16, 64, 48);
    expect(rects.map((r) => [r.x, r.y])).toEqual([
      [32, 16],
      [48, 16],
      [0, 32],
    ]);
  });

  it("báo lỗi khi ảnh không đủ frame", () => {
    expect(() => frameRects(spec({ frames: 5 }), 32, 32, 128, 32)).toThrow("chỉ còn 4");
    expect(() => frameRects(spec({ row: 1 }), 32, 32, 128, 32)).toThrow("hàng 1");
  });
});

describe("frameIndex", () => {
  it("lặp lại khi loop", () => {
    expect(frameIndex(4, 8, true, 0)).toBe(0);
    expect(frameIndex(4, 8, true, 0.26)).toBe(2);
    expect(frameIndex(4, 8, true, 0.5)).toBe(0);
  });

  it("dừng ở frame cuối khi không loop", () => {
    expect(frameIndex(3, 6, false, 10)).toBe(2);
  });
});
