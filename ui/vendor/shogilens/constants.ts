import type { BoardStyle, PieceStyle } from "../../lib/ipc/appSettings";
import type { ColorData } from "./types";

/** 駒種から表示用漢字へのマッピング */
export const PIECE_KANJI: Record<string, string> = {
  P: "歩",
  L: "香",
  N: "桂",
  S: "銀",
  G: "金",
  B: "角",
  R: "飛",
  K: "玉",
};

/** 成駒の表示用漢字 */
export const PROMOTED_PIECE_KANJI: Record<string, string> = {
  P: "と",
  L: "杏",
  N: "圭",
  S: "全",
  B: "馬",
  R: "龍",
};

/** 持ち駒の表示順序（飛車から歩の順） */
export const HAND_PIECE_ORDER: string[] = ["R", "B", "G", "S", "N", "L", "P"];

/** 段の漢数字表記 */
export const RANK_KANJI: string[] = ["", "一", "二", "三", "四", "五", "六", "七", "八", "九"];

// --- SVG レイアウト定数 ---

/** 1マスのサイズ（px） */
export const CELL_SIZE = 44;

/**
 * 盤面外周の余白。
 * 4 辺は同じ値に揃える。
 */
const BOARD_HORIZONTAL_PADDING = 14;
const BOARD_VERTICAL_PADDING_TOTAL = BOARD_HORIZONTAL_PADDING * 2;
const BOARD_VERTICAL_PADDING = BOARD_VERTICAL_PADDING_TOTAL / 2;

/** 盤面のパディング（座標ラベル用） */
export const BOARD_PADDING_TOP = BOARD_VERTICAL_PADDING;
export const BOARD_PADDING_LEFT = BOARD_HORIZONTAL_PADDING;
export const BOARD_PADDING_RIGHT = BOARD_HORIZONTAL_PADDING;
export const BOARD_PADDING_BOTTOM = BOARD_VERTICAL_PADDING;

/** 持ち駒パネルの背景色（フォールバック） */
export const HAND_BG_COLOR = "#e0bc69";

/** 持ち駒パネルの背景テクスチャ CSS */
export const HAND_TEXTURE_CSS = [
  "linear-gradient(90deg, rgba(199,139,76,0.64), rgba(182,124,63,0.64) 60%, rgba(199,139,76,0.64))",
  "repeating-radial-gradient(ellipse at 60% 500%, #b97a38, #b97a38 0.2%, #cc8b48 0.6%, #cc8b48 1%)",
].join(", ");

const LIGHT_HAND_TEXTURE_CSS = [
  "linear-gradient(90deg, rgba(222,184,121,0.76), rgba(206,169,107,0.76) 60%, rgba(222,184,121,0.76))",
  "repeating-radial-gradient(ellipse at 60% 500%, #ce9f5f, #ce9f5f 0.2%, #ddaf71 0.6%, #ddaf71 1%)",
].join(", ");

/** 盤面の描画サイズ（9マス分） */
export const BOARD_SIZE = CELL_SIZE * 9;
export const BOARD_SVG_WIDTH = BOARD_PADDING_LEFT + BOARD_SIZE + BOARD_PADDING_RIGHT;
export const BOARD_SVG_HEIGHT = BOARD_VERTICAL_PADDING_TOTAL + BOARD_SIZE;

/**
 * 盤面本体の幾何情報。
 */
export interface BoardGeometry {
  fileLabelBottomCenterY: number;
  fileLabelTopCenterY: number;
  paddingBottom: number;
  paddingLeft: number;
  paddingRight: number;
  paddingTop: number;
  rankLabelLeftCenterX: number;
  rankLabelRightCenterX: number;
  svgHeight: number;
  svgWidth: number;
}

export function getBoardGeometry(isFlipped: boolean, showCoordinates: boolean): BoardGeometry {
  const _isFlipped = isFlipped;
  const _showCoordinates = showCoordinates;
  void _isFlipped;
  void _showCoordinates;
  const paddingTop = BOARD_VERTICAL_PADDING;
  const paddingBottom = BOARD_VERTICAL_PADDING;
  const svgWidth = BOARD_SVG_WIDTH;
  const svgHeight = BOARD_SVG_HEIGHT;
  const fileLabelTopCenterY = paddingTop / 2 + 1;
  const fileLabelBottomCenterY = svgHeight - fileLabelTopCenterY;
  const rankLabelLeftCenterX = BOARD_PADDING_LEFT / 2;
  const rankLabelRightCenterX = svgWidth - rankLabelLeftCenterX;

  return {
    fileLabelBottomCenterY,
    fileLabelTopCenterY,
    paddingBottom,
    paddingLeft: BOARD_PADDING_LEFT,
    paddingRight: BOARD_PADDING_RIGHT,
    paddingTop,
    rankLabelLeftCenterX,
    rankLabelRightCenterX,
    svgHeight,
    svgWidth,
  };
}

// --- 駒画像パスマッピング ---

type PieceType = "K" | "R" | "B" | "G" | "S" | "N" | "L" | "P";
type PromotablePieceType = "R" | "B" | "S" | "N" | "L" | "P";

/** piece_type → 画像ファイル名部分 */
const PIECE_FILE_NAME: Record<PieceType, string> = {
  K: "king",
  R: "rook",
  B: "bishop",
  G: "gold",
  S: "silver",
  N: "knight",
  L: "lance",
  P: "pawn",
};

/** 成駒の piece_type → 画像ファイル名部分 */
const PROMOTED_PIECE_FILE_NAME: Record<PromotablePieceType, string> = {
  R: "prom_rook",
  B: "prom_bishop",
  S: "prom_silver",
  N: "prom_knight",
  L: "prom_lance",
  P: "prom_pawn",
};

function isPieceType(pieceType: string): pieceType is PieceType {
  return Object.prototype.hasOwnProperty.call(PIECE_FILE_NAME, pieceType);
}

function isPromotablePieceType(pieceType: PieceType): pieceType is PromotablePieceType {
  return Object.prototype.hasOwnProperty.call(PROMOTED_PIECE_FILE_NAME, pieceType);
}

function getPieceImageName(pieceType: string, promoted: boolean): string {
  const normalized = pieceType.toUpperCase();
  if (!isPieceType(normalized)) {
    throw new Error(`[board] Unknown piece_type: ${pieceType}`);
  }

  if (!promoted) {
    return PIECE_FILE_NAME[normalized];
  }

  if (!isPromotablePieceType(normalized)) {
    throw new Error(`[board] Invalid promoted piece_type: ${pieceType}`);
  }
  return PROMOTED_PIECE_FILE_NAME[normalized];
}

/** 表示向きに応じた駒色を返す */
export function getDisplayColor(color: ColorData, isFlipped: boolean): ColorData {
  if (!isFlipped) {
    return color;
  }
  return color === "black" ? "white" : "black";
}

/** 駒種・手番・成り状態から画像パスを取得する */
export function getPieceImagePath(
  pieceType: string,
  color: ColorData,
  promoted: boolean,
  isFlipped = false,
  pieceStyle: PieceStyle = "hitomoji",
): string {
  const displayColor = getDisplayColor(color, isFlipped);
  const name = getPieceImageName(pieceType, promoted);
  const directory = pieceStyle === "hitomojiWood" ? "hitomoji_wood" : "hitomoji";
  return `/pieces/${directory}/${displayColor}_${name}.png`;
}

// --- 色定数 ---

/** 盤面の木目テクスチャ CSS */
export const BOARD_TEXTURE_CSS = [
  "linear-gradient(90deg, rgba(214,156,90,0.58), rgba(198,142,78,0.58) 60%, rgba(214,156,90,0.58))",
  "repeating-radial-gradient(ellipse at 60% 500%, #c88f52, #c88f52 0.2%, #d89f62 0.6%, #d89f62 1%)",
].join(", ");

const LIGHT_BOARD_TEXTURE_CSS = [
  "linear-gradient(90deg, rgba(236,201,141,0.66), rgba(225,190,128,0.66) 60%, rgba(236,201,141,0.66))",
  "repeating-radial-gradient(ellipse at 60% 500%, #e0bc86, #e0bc86 0.2%, #efd2a0 0.6%, #efd2a0 1%)",
].join(", ");

export const COLORS = {
  /** 盤面の背景色（フォールバック） */
  boardBg: "#edd08a",
  /** 盤面の罫線 */
  boardLine: "#4a3520",
  /** 選択中のマス */
  selectedSquare: "rgba(70, 130, 180, 0.5)",
  /** 移動先候補のマス */
  candidateSquare: "rgba(70, 180, 70, 0.4)",
  /** 直前の指し手のマス */
  lastMoveSquare: "rgba(255, 200, 50, 0.4)",
  /** 先手の駒の色 */
  blackPiece: "#1a1a1a",
  /** 後手の駒の色 */
  whitePiece: "#1a1a1a",
  /** 成駒の色 */
  promotedPiece: "#c41e3a",
  /** 座標ラベルの色 */
  coordLabel: "#4a3520",
} as const;

export interface BoardAppearance {
  boardBg: string;
  boardLine: string;
  boardOutlineWidth: number;
  coordLabel: string;
  boardTextureCss: string;
  handBgColor: string;
  handPanelBorder: string;
  handTextureCss: string;
  stageFrameColor: string;
}

const BOARD_APPEARANCES: Record<BoardStyle, BoardAppearance> = {
  standard: {
    boardBg: COLORS.boardBg,
    boardLine: COLORS.boardLine,
    boardOutlineWidth: 1.5,
    coordLabel: COLORS.coordLabel,
    boardTextureCss: BOARD_TEXTURE_CSS,
    handBgColor: HAND_BG_COLOR,
    handPanelBorder: "rgba(74, 53, 32, 0.22)",
    handTextureCss: HAND_TEXTURE_CSS,
    stageFrameColor: "#e6c378",
  },
  light: {
    boardBg: "#f2d7a0",
    boardLine: "#5f4625",
    boardOutlineWidth: 1.5,
    coordLabel: "#5f4625",
    boardTextureCss: LIGHT_BOARD_TEXTURE_CSS,
    handBgColor: "#e3c683",
    handPanelBorder: "rgba(95, 70, 37, 0.2)",
    handTextureCss: LIGHT_HAND_TEXTURE_CSS,
    stageFrameColor: "#e8cc91",
  },
};

export function getBoardAppearance(boardStyle: BoardStyle): BoardAppearance {
  return BOARD_APPEARANCES[boardStyle];
}
