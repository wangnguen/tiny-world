// Icon nhỏ của cửa sổ cài đặt, vẽ bằng SVG để không phải tải file ảnh.

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

export function InfoIcon() {
  return (
    <svg className="icon icon--info" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="9.5" />
      <path d="M12 10.5v6M12 7.2v.1" />
    </svg>
  );
}
