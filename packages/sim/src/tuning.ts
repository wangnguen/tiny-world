/** Thông số hành vi. Đơn vị: CSS pixel và giây. */
export const TUNING = {
  walkSpeed: 30,
  runSpeed: 110,
  /** Thời lượng ngẫu nhiên [min, max] của mỗi lượt đứng / đi / chạy. */
  idleTime: [2, 5],
  walkTime: [2, 6],
  runTime: [1, 2.5],
  /** Xác suất đổi hướng khi bắt đầu đi hoặc chạy. */
  turnChance: 0.4,
  gravity: 2000,
  /** Chạm đất nhanh hơn mức này thì nảy lên; mỗi lần nảy giữ lại `restitution` vận tốc. */
  bounceSpeed: 350,
  restitution: 0.4,
  /** Vận tốc ngang còn lại sau mỗi lần nảy. */
  groundFriction: 0.6,
  /** Chạm đất nhanh hơn mức này thì choáng, tương đương thả từ độ cao khoảng 500 px. */
  dizzySpeed: 1400,
  /** Va vào tường hoặc trần thì bật lại, giữ lại phần này vận tốc. */
  wallBounce: 0.5,
  maxThrowSpeed: 3000,
  reactTime: 0.5,
  /** Click cách lần click trước ít hơn khoảng này là click dồn dập: không nhảy thêm. */
  pokeCooldown: 0.4,
  /** Độ cao cú nhảy khi bị click. */
  hopHeight: 20,
  /** Nhảy xong thì chạy (còn lại là đi) với xác suất này. */
  runAfterPoke: 0.5,
  landTime: 0.25,
  dizzyTime: 2.5,
  /** Không ai click hoặc kéo trong khoảng này thì buồn ngủ: xuống taskbar rồi ngủ. */
  sleepAfter: 180,

  // Cửa sổ.
  /** Mép cửa sổ phải cách trần ít nhất chừng này lần chiều cao pet thì mới đứng được. */
  headroom: 0.8,
  /** Điểm chân cách đầu mép cửa sổ ít nhất chừng này lần bề ngang pet. */
  ledgeMargin: 0.2,
  /** Tới đầu mép cửa sổ: quay lại, ngồi mép, còn lại là xuống. */
  edgeTurn: 0.4,
  edgePerch: 0.35,
  perchTime: [4, 10],
  /** Ngồi mép xong thì quay vào đi tiếp, còn lại là xuống. */
  perchLeaveTurn: 0.6,
  /** Bước khỏi mép khi không nhảy hay leo xuống được: vận tốc ngang, độ nảy lên. */
  stepOffSpeed: 70,
  stepOffHop: 150,
  /** Đi tới một chỗ nhất định (chân tường, chỗ không bị che) quá lâu thì bỏ. */
  goalTime: 15,

  // Phản ứng với cửa sổ.
  /** Cửa sổ đang bị kéo (giữ nguyên cỡ) lại gần pet trong khoảng này thì chạy trốn, chạy trong `fleeTime`. */
  fleeRange: 90,
  fleeTime: [1, 2],
  /** Cửa sổ bị đóng cách pet trong khoảng này thì ăn mừng (xác suất `cheerChance`): nhảy cẫng `cheerHops` cái. */
  cheerRange: 400,
  cheerChance: 0.8,
  cheerHops: 2,
  /** Ăn mừng xong thì chừng này giây sau mới ăn mừng tiếp. */
  cheerCooldown: 15,

  // Con trỏ.
  /** Đang đứng mà con trỏ ở trong khoảng này (tính từ giữa thân) thì quay về phía con trỏ. */
  lookRange: 300,
  /** Con trỏ lệch ngang ít hơn chừng này thì không quay (con trỏ ở ngay trên đầu). */
  lookDeadZone: 12,
  /** Hai lần quay đầu theo con trỏ cách nhau ít nhất chừng này giây. */
  lookTurn: 0.3,

  // Đa màn hình.
  /** Đi tới mép màn hình giáp màn hình khác thì sang bên đó với xác suất này. */
  crossChance: 0.35,
  /** Ra khỏi mép mà overlay không sang màn hình kia sau chừng này giây thì quay lại. */
  crossTimeout: 1.5,

  // Leo.
  climbSpeed: 50,
  climbChance: 0.25,
  /** Tường thấp hơn chừng này thì không leo. */
  minClimb: 40,
  /** Tìm tường để leo trong khoảng này quanh pet. */
  climbSearch: 400,
  /** Khoảng từ điểm chân tới tường lúc leo, tính theo bề ngang pet (PetView đo lại theo sprite). */
  reach: 0.3,
  /** Leo tới đỉnh thì nhún qua mép cao chừng này. */
  mantleArc: 14,
  /**
   * Cửa sổ lơ lửng (đáy cao hơn chỗ đứng): nhảy thẳng lên bám cạnh, lúc bám chân thấp hơn đáy cửa sổ
   * chừng này lần chiều cao pet (tay ôm phần dưới của cạnh). Nhảy cao tối đa `maxGrabJump`.
   */
  grabDepth: 0.5,
  maxGrabJump: 220,

  // Nhảy.
  jumpChance: 0.25,
  maxJumpX: 320,
  maxJumpUp: 170,
  maxJumpDown: 450,
  /** Đỉnh cú nhảy cao hơn điểm cao hơn giữa chỗ đứng và chỗ đáp: `jumpArc` cộng phần theo khoảng cách ngang. */
  jumpArc: 36,
  jumpArcPerPx: 0.1,
  jumpCrouch: 0.12,
  jumpTouchdown: 0.2,
} as const;
