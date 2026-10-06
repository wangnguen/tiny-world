// Icon nhỏ của cửa sổ cài đặt, vẽ bằng SVG để không phải tải file ảnh.

export function PawIcon() {
  return (
    <svg className="icon icon--paw" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 12.5c-3 0-6 3.4-6 5.6 0 1.6 1.4 2.4 3 2.4 1.2 0 2-.6 3-.6s1.8.6 3 .6c1.6 0 3-.8 3-2.4 0-2.2-3-5.6-6-5.6z" />
      <ellipse cx="5" cy="10" rx="1.9" ry="2.4" />
      <ellipse cx="9.2" cy="5.8" rx="1.9" ry="2.5" />
      <ellipse cx="14.8" cy="5.8" rx="1.9" ry="2.5" />
      <ellipse cx="19" cy="10" rx="1.9" ry="2.4" />
    </svg>
  );
}

export function CubeIcon() {
  return (
    <svg className="icon icon--cube" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.8 20 7.2v9.6l-8 4.4-8-4.4V7.2z" />
      <path d="M4 7.2 12 11.6l8-4.4M12 11.6v9.6" />
    </svg>
  );
}

export function RunnerIcon() {
  return (
    <svg className="icon icon--runner" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="16" cy="4" r="2" />
      <path d="M14.5 7.5 11 12l3.5 3-1.5 5.5M11 12l-3.5 2M14.5 7.5l3.5 3 2.5-.5M14.5 7.5 11 7l-2.5 2.5M12.5 14.5 9 19.5H6" />
      <path className="icon__speed" d="M2 9h4M1 12.5h4M2.5 16h3" />
    </svg>
  );
}

export function WindowsIcon() {
  return (
    <svg className="icon icon--windows" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 5.4 10.4 4.3v7H3zM11.7 4.1 21 2.8v8.5h-9.3zM3 12.6h7.4v7.1L3 18.6zM11.7 12.6H21v8.6l-9.3-1.3z" />
    </svg>
  );
}

/** Mũi tên của nút lật trang, `left` là trang trước. */
export function ChevronIcon({ direction }: { direction: "left" | "right" }) {
  return (
    <svg className="icon icon--chevron" viewBox="0 0 24 24" aria-hidden="true">
      <path d={direction === "left" ? "M14.5 6 8.5 12l6 6" : "M9.5 6l6 6-6 6"} />
    </svg>
  );
}

export function InfoIcon() {
  return (
    <svg className="icon icon--info" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="9.5" />
      <path d="M12 10.5v6M12 7.2v.1" />
    </svg>
  );
}

/** Mục Thành phố (tab Thế giới). */
export function PinIcon() {
  return (
    <svg className="icon icon--pin" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 21s-6.5-6.2-6.5-11.2a6.5 6.5 0 0 1 13 0C18.5 14.8 12 21 12 21z" />
      <circle cx="12" cy="9.8" r="2.4" />
    </svg>
  );
}

/** Mục Hiệu ứng (thời tiết, câu nói, con ma). */
export function SparkIcon() {
  return (
    <svg className="icon icon--spark" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M18 6l-2.5 2.5M8.5 15.5 6 18" />
    </svg>
  );
}

/** Mục Xem thử (cho pet gặp ngay một hiệu ứng). */
export function PlayIcon() {
  return (
    <svg className="icon icon--play" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" />
      <path d="M10.5 8.8v6.4l5-3.2z" />
    </svg>
  );
}

/** Mục Lịch sự kiện. */
export function CalendarIcon() {
  return (
    <svg className="icon icon--calendar" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="3.5" y="5" width="17" height="15.5" rx="2.5" />
      <path d="M3.5 10h17M8 3v4M16 3v4M8 14h2M14 14h2M8 17h2" />
    </svg>
  );
}

/** Mục Giờ ngồi máy (tab Sức khoẻ). */
export function ClockIcon() {
  return (
    <svg className="icon icon--clock" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3.5 2" />
    </svg>
  );
}

/** Mục Nhắc nhở (tab Sức khoẻ). */
export function BellIcon() {
  return (
    <svg className="icon icon--bell" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 2h-15z" />
      <path d="M10 20.5a2 2 0 0 0 4 0" />
    </svg>
  );
}

/** Mục Chat với pet. */
export function ChatIcon() {
  return (
    <svg className="icon icon--chat" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4.5 5.5h15v10h-8l-4 3.5v-3.5h-3z" />
      <path d="M8.5 10.5h.1M12 10.5h.1M15.5 10.5h.1" />
    </svg>
  );
}
