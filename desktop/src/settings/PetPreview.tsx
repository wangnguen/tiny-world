import { useEffect, useRef } from "react";
import { frameIndex } from "@tinyworld/core";
import { loadSpriteSet } from "../overlay/sprites";

interface Props {
  /** Nhân vật đang chọn (`Settings.pet`). */
  pet: string | null;
  /** Chỗ đặt chân pet (CSS pixel trong khung chứa). */
  footX: number;
  footY: number;
  /** Chiều cao frame khi vẽ (CSS pixel); ảnh nhỏ phóng theo số nguyên gần nhất. */
  height: number;
}

/** Pet của người dùng (cùng sprite pack với overlay) đứng trên đồi, chạy animation idle, quay mặt sang trái. */
export function PetPreview({ pet, footX, footY, height: targetHeight }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let timer = 0;
    let cancelled = false;
    void loadSpriteSet(pet).then((sprite) => {
      const canvas = ref.current;
      const ctx = canvas?.getContext("2d");
      if (cancelled || !canvas || !ctx) return;
      const { frameWidth: width, frameHeight: height, anchor } = sprite;
      // Ảnh lớn vẫn thu về đúng khung; ảnh nhỏ phóng theo số nguyên để giữ nét pixel.
      const fit = targetHeight / height;
      const scale = fit >= 1 ? Math.round(fit) : fit;
      const idle = sprite.animations.idle;
      const flip = sprite.facing === "right";
      const anchorX = flip ? width - anchor.x : anchor.x;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(width * scale * dpr);
      canvas.height = Math.round(height * scale * dpr);
      Object.assign(canvas.style, {
        width: `${width * scale}px`,
        height: `${height * scale}px`,
        left: `${footX - anchorX * scale}px`,
        top: `${footY - anchor.y * scale}px`,
      });

      const start = performance.now();
      const draw = () => {
        const time = (performance.now() - start) / 1000;
        const frame = idle.frames[frameIndex(idle.frames.length, idle.fps, true, time)];
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.imageSmoothingEnabled = !sprite.pixelArt;
        if (flip) ctx.setTransform(-1, 0, 0, 1, canvas.width, 0);
        ctx.drawImage(idle.image, frame.x, frame.y, frame.width, frame.height, 0, 0, canvas.width, canvas.height);
      };
      draw();
      timer = window.setInterval(draw, 1000 / idle.fps);
    });
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [pet, footX, footY, targetHeight]);

  return <canvas ref={ref} className="hero__pet" aria-hidden="true" />;
}
