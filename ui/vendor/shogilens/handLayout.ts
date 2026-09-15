import { CELL_SIZE, HAND_PIECE_ORDER } from "./constants";
import type { ColorData } from "./types";

export type HandSeatPosition = "top" | "bottom";

export interface HandPieceBounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export const HAND_IMAGE_SOURCE_WIDTH = 140;
export const HAND_IMAGE_SOURCE_HEIGHT = 148;
export const HAND_PANEL_HORIZONTAL_PADDING = 6;
export const HAND_PANEL_VERTICAL_PADDING = 5;
export const HAND_VISIBLE_GAP = 7;

const BLACK_HAND_PIECE_BOUNDS: Record<string, HandPieceBounds> = {
  R: { left: 7, top: 6, right: 3, bottom: 0 },
  B: { left: 8, top: 6, right: 2, bottom: 0 },
  G: { left: 11, top: 10, right: 4, bottom: 2 },
  S: { left: 11, top: 11, right: 4, bottom: 1 },
  N: { left: 14, top: 15, right: 9, bottom: 2 },
  L: { left: 16, top: 16, right: 7, bottom: 1 },
  P: { left: 23, top: 18, right: 12, bottom: 2 },
};

const WHITE_HAND_PIECE_BOUNDS: Record<string, HandPieceBounds> = {
  R: { left: 0, top: 0, right: 2, bottom: 3 },
  B: { left: 8, top: 0, right: 1, bottom: 3 },
  G: { left: 12, top: 5, right: 3, bottom: 7 },
  S: { left: 12, top: 5, right: 3, bottom: 7 },
  N: { left: 15, top: 8, right: 9, bottom: 9 },
  L: { left: 17, top: 9, right: 7, bottom: 8 },
  P: { left: 23, top: 11, right: 11, bottom: 10 },
};

export function getHandPanelHorizontalPadding(scale: number): number {
  return HAND_PANEL_HORIZONTAL_PADDING * scale;
}

export function getHandPanelVerticalPadding(scale: number): number {
  return HAND_PANEL_VERTICAL_PADDING * scale;
}

export function getHandPanelHeight(scale: number): number {
  return CELL_SIZE * scale + getHandPanelVerticalPadding(scale) * 2;
}

export function getHandVisibleGap(scale: number): number {
  return Math.max(2, HAND_VISIBLE_GAP * scale);
}

export function getHandContentWidth(
  pieceOrder: readonly string[],
  displayColor: ColorData,
  pieceSize: number,
  visibleGap: number,
): number {
  const pieceLefts = getHandPieceLefts(pieceOrder, displayColor, pieceSize, visibleGap);
  return pieceLefts[pieceLefts.length - 1] + pieceSize;
}

export function getStandardHandPanelWidth(scale: number, visibleGap: number): number {
  const pieceSize = CELL_SIZE * scale;
  const horizontalPadding = getHandPanelHorizontalPadding(scale);
  const topContentWidth = getHandContentWidth(
    [...HAND_PIECE_ORDER].reverse(),
    "white",
    pieceSize,
    visibleGap,
  );
  const bottomContentWidth = getHandContentWidth(HAND_PIECE_ORDER, "black", pieceSize, visibleGap);

  return Math.max(topContentWidth, bottomContentWidth) + horizontalPadding * 2;
}

export function getHandPieceBounds(pieceType: string, displayColor: ColorData): HandPieceBounds {
  if (displayColor === "white") {
    return WHITE_HAND_PIECE_BOUNDS[pieceType] ?? WHITE_HAND_PIECE_BOUNDS.P;
  }
  return BLACK_HAND_PIECE_BOUNDS[pieceType] ?? BLACK_HAND_PIECE_BOUNDS.P;
}

export function getHandPieceLefts(
  pieceOrder: readonly string[],
  displayColor: ColorData,
  pieceSize: number,
  visibleGap: number,
): number[] {
  const lefts: number[] = [];
  let currentLeft = 0;

  pieceOrder.forEach((pieceType, index) => {
    lefts.push(currentLeft);
    const nextPieceType = pieceOrder[index + 1];
    if (!nextPieceType) {
      return;
    }

    const currentBounds = getHandPieceBounds(pieceType, displayColor);
    const nextBounds = getHandPieceBounds(nextPieceType, displayColor);
    currentLeft +=
      pieceSize -
      scaleHorizontalInset(currentBounds.right, pieceSize) -
      scaleHorizontalInset(nextBounds.left, pieceSize) +
      visibleGap;
  });

  return lefts;
}

export function getHandPieceTop(
  pieceType: string,
  displayColor: ColorData,
  _seatPosition: HandSeatPosition,
  pieceSize: number,
  scale: number,
): number {
  const paddingY = getHandPanelVerticalPadding(scale);
  const bounds = getHandPieceBounds(pieceType, displayColor);
  const targetLowerEdgeInset = scaleVerticalInset(
    getMaxLowerEdgeSourceInset(displayColor),
    pieceSize,
  );
  return paddingY + scaleVerticalInset(bounds.bottom, pieceSize) - targetLowerEdgeInset;
}

export function getHandPieceVisibleBoardGap(
  pieceType: string,
  displayColor: ColorData,
  seatPosition: HandSeatPosition,
  pieceSize: number,
  scale: number,
): number {
  const panelHeight = getHandPanelHeight(scale);
  const pieceTop = getHandPieceTop(pieceType, displayColor, seatPosition, pieceSize, scale);
  const bounds = getHandPieceBounds(pieceType, displayColor);

  if (seatPosition === "top") {
    return panelHeight - (pieceTop + pieceSize - scaleVerticalInset(bounds.bottom, pieceSize));
  }

  return pieceTop + scaleVerticalInset(bounds.top, pieceSize);
}

export function scaleHorizontalInset(sourceInset: number, pieceSize: number): number {
  return (sourceInset / HAND_IMAGE_SOURCE_WIDTH) * pieceSize;
}

export function scaleVerticalInset(sourceInset: number, pieceSize: number): number {
  return (sourceInset / HAND_IMAGE_SOURCE_HEIGHT) * pieceSize;
}

function getMaxLowerEdgeSourceInset(displayColor: ColorData): number {
  const boundsByPiece =
    displayColor === "white" ? WHITE_HAND_PIECE_BOUNDS : BLACK_HAND_PIECE_BOUNDS;
  return Math.max(...Object.values(boundsByPiece).map((bounds) => bounds.bottom));
}
