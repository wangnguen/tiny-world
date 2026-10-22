//! Thông số hành vi, giống `TUNING` của packages/sim. Đơn vị: CSS pixel và giây.

pub const WALK_SPEED: f64 = 30.0;
pub const RUN_SPEED: f64 = 110.0;
/// Thời lượng ngẫu nhiên [min, max] của mỗi lượt đứng / đi / chạy.
pub const IDLE_TIME: (f64, f64) = (2.0, 5.0);
pub const WALK_TIME: (f64, f64) = (2.0, 6.0);
pub const RUN_TIME: (f64, f64) = (1.0, 2.5);
/// Xác suất đổi hướng khi bắt đầu đi hoặc chạy.
pub const TURN_CHANCE: f64 = 0.4;
pub const GRAVITY: f64 = 2000.0;
/// Chạm đất nhanh hơn mức này thì nảy lên; mỗi lần nảy giữ lại `RESTITUTION` vận tốc.
pub const BOUNCE_SPEED: f64 = 350.0;
pub const RESTITUTION: f64 = 0.4;
/// Vận tốc ngang còn lại sau mỗi lần nảy.
pub const GROUND_FRICTION: f64 = 0.6;
/// Chạm đất nhanh hơn mức này thì choáng, tương đương thả từ độ cao khoảng 500 px.
pub const DIZZY_SPEED: f64 = 1400.0;
/// Va vào tường hoặc trần thì bật lại, giữ lại phần này vận tốc.
pub const WALL_BOUNCE: f64 = 0.5;
pub const MAX_THROW_SPEED: f64 = 3000.0;
pub const REACT_TIME: f64 = 0.5;
/// Click cách lần click trước ít hơn khoảng này là click dồn dập: không nhảy thêm.
pub const POKE_COOLDOWN: f64 = 0.4;
/// Độ cao cú nhảy khi bị click.
pub const HOP_HEIGHT: f64 = 20.0;
/// Nhảy xong thì chạy (còn lại là đi) với xác suất này.
pub const RUN_AFTER_POKE: f64 = 0.5;
pub const LAND_TIME: f64 = 0.25;
pub const DIZZY_TIME: f64 = 2.5;
/// Không ai click hoặc kéo trong khoảng này thì buồn ngủ: xuống taskbar rồi ngủ.
pub const SLEEP_AFTER: f64 = 180.0;
/// Nhiều pet: mỗi con khác đang đi lại (đi, chạy, leo, nhảy) làm xác suất con này đi lại giảm chừng này phần; 2 con
/// kia đang đi thì con này chỉ còn 30% khả năng đi như bình thường.
pub const GROUP_CALM: f64 = 0.35;
/// Im lặng (tray): xác suất tự đi lại (đi, chạy, leo, nhảy) chỉ còn chừng này phần.
pub const QUIET_CALM: f64 = 0.25;

// Ngày/đêm.
/// Ban đêm đi lại chậm hơn: tốc độ nhân với chừng này.
pub const NIGHT_PACE: f64 = 0.75;
/// Ban đêm buồn ngủ sớm hơn `SLEEP_AFTER`.
pub const SLEEP_AFTER_NIGHT: f64 = 90.0;

// Nói (speech bubble).
/// Một câu hiện trong chừng này giây.
pub const SPEECH_TIME: f64 = 4.0;
/// Câu nói cho vui (chào nhau, thời tiết): cả nhóm chừng này giây mới nói một câu, tối đa 4 câu mỗi giờ.
pub const CHAT_GAP: f64 = 900.0;

// Chơi với nhau.
/// Đi tới sát con khác (khoảng giữa hai con dưới chừng này lần nửa bề ngang cộng lại) thì dừng lại chào.
pub const MEET_REACH: f64 = 0.9;
/// Chào nhau xong thì đứng quay mặt vào nhau chừng này giây.
pub const MEET_TIME: (f64, f64) = (2.0, 4.0);
/// Một con chừng này giây mới dừng lại chào một lần.
pub const MEET_COOLDOWN: f64 = 45.0;
/// Chào nhau thì nói một câu với xác suất này (vẫn theo `CHAT_GAP`).
pub const MEET_CHAT_CHANCE: f64 = 0.5;
/// Chào nhau xong thì đi cùng nhau một đoạn với xác suất này, trong chừng này giây.
pub const STROLL_CHANCE: f64 = 0.4;
pub const STROLL_TIME: (f64, f64) = (3.0, 7.0);
/// Buồn ngủ mà có con đang ngủ dưới đất trong khoảng này thì đi tới nằm cạnh.
pub const NAP_RANGE: f64 = 360.0;
/// Nằm cạnh nhau: hai điểm chân cách nhau chừng này lần nửa bề ngang cộng lại (hơi chồng lên nhau).
pub const NAP_GAP: f64 = 0.8;

// Cửa sổ.
/// Mép cửa sổ phải cách trần ít nhất chừng này lần chiều cao pet thì mới đứng được.
pub const HEADROOM: f64 = 0.8;
/// Điểm chân cách đầu mép cửa sổ ít nhất chừng này lần bề ngang pet.
pub const LEDGE_MARGIN: f64 = 0.2;
/// Tới đầu mép cửa sổ: quay lại, ngồi mép, còn lại là xuống.
pub const EDGE_TURN: f64 = 0.4;
pub const EDGE_PERCH: f64 = 0.35;
pub const PERCH_TIME: (f64, f64) = (4.0, 10.0);
/// Ngồi mép xong thì quay vào đi tiếp, còn lại là xuống.
pub const PERCH_LEAVE_TURN: f64 = 0.6;
/// Bước khỏi mép khi không nhảy hay leo xuống được: vận tốc ngang, độ nảy lên.
pub const STEP_OFF_SPEED: f64 = 70.0;
pub const STEP_OFF_HOP: f64 = 150.0;
/// Đi tới một chỗ nhất định (chân tường, chỗ không bị che) quá lâu thì bỏ.
pub const GOAL_TIME: f64 = 15.0;

// Phản ứng với cửa sổ.
/// Cửa sổ đang bị kéo (giữ nguyên cỡ) lại gần pet trong khoảng này thì chạy trốn, chạy trong `FLEE_TIME`.
pub const FLEE_RANGE: f64 = 90.0;
pub const FLEE_TIME: (f64, f64) = (1.0, 2.0);
/// Cửa sổ bị đóng cách pet trong khoảng này thì ăn mừng (xác suất `CHEER_CHANCE`): nhảy cẫng `CHEER_HOPS` cái.
pub const CHEER_RANGE: f64 = 400.0;
pub const CHEER_CHANCE: f64 = 0.8;
pub const CHEER_HOPS: u32 = 2;
/// Ăn mừng xong thì chừng này giây sau mới ăn mừng tiếp.
pub const CHEER_COOLDOWN: f64 = 15.0;

// Con trỏ.
/// Đang đứng mà con trỏ ở trong khoảng này (tính từ giữa thân) thì quay về phía con trỏ.
pub const LOOK_RANGE: f64 = 300.0;
/// Con trỏ lệch ngang ít hơn chừng này thì không quay (con trỏ ở ngay trên đầu).
pub const LOOK_DEAD_ZONE: f64 = 12.0;
/// Hai lần quay đầu theo con trỏ cách nhau ít nhất chừng này giây.
pub const LOOK_TURN: f64 = 0.3;

// Đa màn hình.
/// Đi tới mép màn hình giáp màn hình khác thì sang bên đó với xác suất này.
pub const CROSS_CHANCE: f64 = 0.35;
/// Ra khỏi mép mà overlay không sang màn hình kia sau chừng này giây thì quay lại.
pub const CROSS_TIMEOUT: f64 = 1.5;

// Leo.
pub const CLIMB_SPEED: f64 = 50.0;
pub const CLIMB_CHANCE: f64 = 0.25;
/// Tường thấp hơn chừng này thì không leo.
pub const MIN_CLIMB: f64 = 40.0;
/// Tìm tường để leo trong khoảng này quanh pet.
pub const CLIMB_SEARCH: f64 = 400.0;
/// Khoảng từ điểm chân tới tường lúc leo, tính theo bề ngang pet (phần vẽ đo lại theo sprite).
pub const REACH: f64 = 0.3;
/// Leo tới đỉnh thì nhún qua mép cao chừng này.
pub const MANTLE_ARC: f64 = 14.0;
/// Cửa sổ lơ lửng (đáy cao hơn chỗ đứng): nhảy thẳng lên bám cạnh, lúc bám chân thấp hơn đáy cửa sổ chừng này lần
/// chiều cao pet (tay ôm phần dưới của cạnh). Nhảy cao tối đa `MAX_GRAB_JUMP`.
pub const GRAB_DEPTH: f64 = 0.5;
pub const MAX_GRAB_JUMP: f64 = 220.0;

// Nhảy.
pub const JUMP_CHANCE: f64 = 0.25;
pub const MAX_JUMP_X: f64 = 320.0;
pub const MAX_JUMP_UP: f64 = 170.0;
pub const MAX_JUMP_DOWN: f64 = 450.0;
/// Đỉnh cú nhảy cao hơn điểm cao hơn giữa chỗ đứng và chỗ đáp: `JUMP_ARC` cộng phần theo khoảng cách ngang.
pub const JUMP_ARC: f64 = 36.0;
pub const JUMP_ARC_PER_PX: f64 = 0.1;
pub const JUMP_CROUCH: f64 = 0.12;
pub const JUMP_TOUCHDOWN: f64 = 0.2;
