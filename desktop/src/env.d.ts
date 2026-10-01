interface ImportMetaEnv {
  /**
   * Bản build tự động khi push lên `main` (workflow Build): "<số lần chạy> · <commit>". Bản Release và
   * bản dev không có.
   */
  readonly VITE_BUILD?: string;
}
