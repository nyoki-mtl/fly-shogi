/** マス目の座標（筋: 1-9, 段: 1-9） */
export interface SquareData {
  file: number;
  rank: number;
}

/** 手番 */
export type ColorData = "black" | "white";

/** 局面妥当性ステータス */
export type PositionValidationStatus = "ready" | "incomplete" | "invalid";

/** 局面妥当性 issue */
export interface PositionIssue {
  code: string;
  message: string;
}

/** 局面妥当性情報 */
export interface PositionValidation {
  status: PositionValidationStatus;
  issues: PositionIssue[];
}

/** 駒の情報 */
export interface PieceData {
  piece_type: string;
  promoted: boolean;
  color: ColorData;
}

/** 盤上の駒情報 */
export interface BoardPieceData {
  square: SquareData;
  piece: PieceData;
}

/** 持ち駒の1エントリ */
export interface HandPieceEntry {
  piece_type: string;
  count: number;
}

/** 持ち駒の情報 */
export interface HandData {
  pieces: HandPieceEntry[];
}

/** 局面の完全な情報 */
export interface PositionData {
  sfen: string;
  turn: ColorData;
  board: BoardPieceData[];
  black_hand: HandData;
  white_hand: HandData;
  ply: number;
  in_check: boolean;
  validation: PositionValidation;
}

/** 指し手のデータ */
export interface MoveData {
  usi: string;
  from: SquareData | null;
  to: SquareData;
  promotion: boolean;
  is_drop: boolean;
  drop_piece_type: string | null;
}

/** 盤面の選択状態 */
export interface SelectionState {
  /** 選択中のマス（盤上の駒を選択中） */
  square: SquareData | null;
  /** 選択中の持ち駒（駒打ち操作中） */
  handPiece: { pieceType: string; color: ColorData } | null;
  /** 移動先の候補 */
  candidates: MoveData[];
}

/** 成り選択ダイアログの状態 */
export interface PromotionChoice {
  /** 成りの指し手 */
  promoteMove: MoveData;
  /** 不成の指し手 */
  noPromoteMove: MoveData;
}

/** 局面編集用プリセット */
export interface PositionPreset {
  id: string;
  sfen: string;
}

/** 局面編集用の hand ソース */
export interface PositionEditHandSource {
  piece_type: string;
  color: ColorData;
}

/** 局面編集用の hand 目的地 */
export interface PositionEditHandTarget {
  color: ColorData;
}

/** 局面編集用の box ソース */
export interface PositionEditBoxSource {
  box: true;
  piece_type: string;
}

/** 局面編集用の box 目的地 */
export interface PositionEditBoxTarget {
  box: true;
  piece_type: string;
}

/** 局面編集の移動元 */
export type PositionEditSource = SquareData | PositionEditHandSource | PositionEditBoxSource;

/** 局面編集の移動先 */
export type PositionEditTarget = SquareData | PositionEditHandTarget | PositionEditBoxTarget;

/** 局面編集の move 操作 */
export interface PositionEditMove {
  from: PositionEditSource;
  to: PositionEditTarget;
}

/** 局面編集の変更要求 */
export type PositionEditChange =
  | { move: PositionEditMove }
  | { rotate: SquareData }
  | { rotateBoard: true };

/** ドラッグのソース種別 */
export type DragSource =
  | { type: "board"; file: number; rank: number }
  | { type: "hand"; pieceType: string; color: ColorData }
  | { type: "box"; pieceType: string };

/** ドラッグ状態 */
export interface DragState {
  /** pointerdown 済みでまだ閾値未到達 */
  pending: boolean;
  /** ドラッグ確定済み（ゴースト表示中） */
  active: boolean;
  /** 追跡中の pointerId */
  pointerId: number | null;
  /** ドラッグ元 */
  source: DragSource | null;
  /** ドラッグ中の駒情報 */
  piece: PieceData | null;
  /**
   * ゴーストの一辺（px, CSS ピクセル）。掴んだ要素の実測値。
   * null = 実測できなかった（呼び出し側が `CELL_SIZE * scale` で代替する）。
   */
  ghostSize: number | null;
  /** ゴースト座標（ビューポート基準） */
  ghostX: number;
  ghostY: number;
  /** pointerdown 座標（閾値判定用） */
  startX: number;
  startY: number;
}
