import type { WindowInfo } from "@tinyworld/core";
import { describe, expect, it } from "vitest";
import { Terrain, subtract } from "./terrain";

const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

function win(id: number, x: number, y: number, width: number, height: number): WindowInfo {
  return { id, rect: { x, y, width, height } };
}

describe("mép cửa sổ", () => {
  it("mép trên là chỗ đứng, cắt theo màn hình", () => {
    const terrain = new Terrain(bounds, [win(1, -100, 300, 400, 200)]);
    expect(terrain.ledge(1)).toEqual({ id: 1, y: 300, from: 0, to: 300, open: [{ from: 0, to: 300 }] });
  });

  it("không có mép khi cửa sổ chạm trần (phóng to) hoặc nằm dưới mặt đất", () => {
    const terrain = new Terrain(bounds, [win(1, 0, 0, 1000, 700), win(2, 100, 700, 300, 200)]);
    expect(terrain.ledges).toEqual([]);
    expect(terrain.ledge(1)).toBeUndefined();
  });

  it("phần bị cửa sổ nằm trên che thì không đứng được", () => {
    // Cửa sổ 1 nằm trên, đè lên giữa mép trên của cửa sổ 2.
    const terrain = new Terrain(bounds, [win(1, 300, 200, 200, 300), win(2, 100, 300, 600, 300)]);
    expect(terrain.ledge(2)?.open).toEqual([
      { from: 100, to: 300 },
      { from: 500, to: 700 },
    ]);
    // Cửa sổ nằm trên thì mép của nó không bị cửa sổ nằm dưới che.
    expect(terrain.ledge(1)?.open).toEqual([{ from: 300, to: 500 }]);
  });

  it("cửa sổ nằm trên nhưng ở hẳn phía trên mép thì không che mép", () => {
    const terrain = new Terrain(bounds, [win(1, 300, 100, 200, 150), win(2, 100, 300, 600, 300)]);
    expect(terrain.ledge(2)?.open).toEqual([{ from: 100, to: 700 }]);
  });

  it("chỉ cửa sổ nằm trên mới che được pet đứng trên cửa sổ", () => {
    const terrain = new Terrain(bounds, [win(1, 0, 0, 50, 50), win(2, 100, 300, 600, 300), win(3, 0, 0, 20, 20)]);
    expect(terrain.above(2)).toEqual([{ x: 0, y: 0, width: 50, height: 50 }]);
    expect(terrain.above(1)).toEqual([]);
    expect(terrain.covered(2, 10, 10)).toBe(true);
    expect(terrain.covered(1, 10, 10)).toBe(false);
  });
});

describe("cạnh cửa sổ", () => {
  it("hai cạnh bên là tường, cắt theo trần và mặt đất", () => {
    const terrain = new Terrain(bounds, [win(1, 200, 400, 300, 500)]);
    expect(terrain.wall(1, -1)).toEqual({ id: 1, side: -1, x: 200, from: 400, to: 700, open: [{ from: 400, to: 700 }] });
    expect(terrain.wall(1, 1)?.x).toBe(500);
  });

  it("cạnh nằm ngoài màn hình thì không leo được", () => {
    const terrain = new Terrain(bounds, [win(1, -50, 400, 300, 200)]);
    expect(terrain.wall(1, -1)).toBeUndefined();
    expect(terrain.wall(1, 1)).toBeDefined();
  });

  it("phần cạnh bị cửa sổ nằm trên che thì không leo được", () => {
    const terrain = new Terrain(bounds, [win(1, 100, 450, 150, 100), win(2, 200, 400, 300, 300)]);
    expect(terrain.wall(2, -1)?.open).toEqual([
      { from: 400, to: 450 },
      { from: 550, to: 700 },
    ]);
  });
});

describe("subtract", () => {
  it("cắt bỏ các đoạn chồng lên nhau", () => {
    expect(subtract([{ from: 0, to: 100 }], [{ from: 20, to: 40 }, { from: 30, to: 60 }, { from: 90, to: 200 }])).toEqual([
      { from: 0, to: 20 },
      { from: 60, to: 90 },
    ]);
  });
});
