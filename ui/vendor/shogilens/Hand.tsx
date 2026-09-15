import type React from "react";
import type { BoardStyle, PieceStyle } from "../../lib/ipc/appSettings";
import {
  BOARD_PADDING_LEFT,
  BOARD_PADDING_RIGHT,
  BOARD_SIZE,
  CELL_SIZE,
  HAND_PIECE_ORDER,
  PIECE_KANJI,
  getBoardAppearance,
  getPieceImagePath,
} from "./constants";
import {
  getHandPieceBounds,
  getHandContentWidth,
  getHandPanelHeight,
  getHandPanelHorizontalPadding,
  getHandPieceLefts,
  getHandPieceTop,
  getStandardHandPanelWidth,
  getHandVisibleGap,
  HAND_IMAGE_SOURCE_HEIGHT,
  HAND_IMAGE_SOURCE_WIDTH,
  type HandSeatPosition,
} from "./handLayout";
import type { ColorData, HandData } from "./types";

interface HandProps {
  hand: HandData;
  color: ColorData;
  align?: "start" | "end";
  displayColor?: ColorData;
  pieceOrder?: readonly string[];
  seatPosition?: HandSeatPosition;
  pieceStyle: PieceStyle;
  boardStyle: BoardStyle;
  selectedPieceType: string | null;
  onPieceClick: (pieceType: string) => void;
  onPiecePointerDown: (pieceType: string, e: React.PointerEvent) => void;
  onPiecePointerUp?: (pieceType: string, e: React.PointerEvent) => void;
  onPanelClick?: () => void;
  panelRef?: React.Ref<HTMLDivElement>;
  /** フレームスケール（1.0 = 参照サイズ） */
  scale?: number;
  flatStyle?: boolean;
  forcedWidth?: number;
  fullBoardWidthOverride?: number;
  suppressBorderTop?: boolean;
  suppressBorderBottom?: boolean;
}

const HAND_COUNT_CORNER_NUDGE = 1;
const HAND_COUNT_OFFSET_X = 1.7;
const HAND_COUNT_OFFSET_Y = 1.7;
/** 持ち駒パネルコンポーネント */
export function Hand({
  hand,
  color,
  align,
  displayColor,
  pieceOrder,
  seatPosition,
  pieceStyle,
  boardStyle,
  selectedPieceType,
  onPieceClick,
  onPiecePointerDown,
  onPiecePointerUp,
  onPanelClick,
  panelRef,
  scale = 1,
  flatStyle = false,
  forcedWidth,
  fullBoardWidthOverride,
  suppressBorderTop = false,
  suppressBorderBottom = false,
}: HandProps) {
  const countByPieceType = new Map(hand.pieces.map((piece) => [piece.piece_type, piece.count]));
  const resolvedPieceOrder =
    pieceOrder ?? (color === "black" ? HAND_PIECE_ORDER : [...HAND_PIECE_ORDER].reverse());
  const resolvedDisplayColor = displayColor ?? color;
  const resolvedAlign = align ?? (color === "black" ? "end" : "start");
  const resolvedSeatPosition = seatPosition ?? (resolvedAlign === "start" ? "top" : "bottom");
  const appearance = getBoardAppearance(boardStyle);
  const pieceSize = CELL_SIZE * scale;
  const paddingX = getHandPanelHorizontalPadding(scale);
  const visibleGap = getHandVisibleGap(scale);
  const panelHeight = getHandPanelHeight(scale);
  const standardPanelWidth = getStandardHandPanelWidth(scale, visibleGap);
  const pieceLefts = getHandPieceLefts(
    resolvedPieceOrder,
    resolvedDisplayColor,
    pieceSize,
    visibleGap,
  );
  const contentWidth = getHandContentWidth(
    resolvedPieceOrder,
    resolvedDisplayColor,
    pieceSize,
    visibleGap,
  );
  const fullBoardWidth =
    fullBoardWidthOverride ?? (BOARD_PADDING_LEFT + BOARD_SIZE + BOARD_PADDING_RIGHT) * scale;
  const panelWidth = Math.max(standardPanelWidth, contentWidth + paddingX * 2, forcedWidth ?? 0);
  const panelOffsetX = resolvedAlign === "end" ? Math.max(0, fullBoardWidth - panelWidth) : 0;
  const textureTransform = resolvedSeatPosition === "top" ? "rotate(180deg)" : undefined;

  return (
    <div
      ref={panelRef}
      data-testid={`hand-panel-${color}`}
      onClick={() => {
        onPanelClick?.();
      }}
      style={{
        position: "relative",
        width: `${panelWidth}px`,
        height: `${panelHeight}px`,
        marginLeft: `${panelOffsetX}px`,
        borderRadius: flatStyle ? "0" : `${Math.max(4, 6 * scale)}px`,
        border: `1px solid ${appearance.handPanelBorder}`,
        borderTopWidth: suppressBorderTop ? "0" : "1px",
        borderBottomWidth: suppressBorderBottom ? "0" : "1px",
        backgroundColor: appearance.handBgColor,
        boxSizing: "border-box",
        overflow: "hidden",
      }}
    >
      <div
        aria-hidden="true"
        data-testid={`hand-panel-background-${color}`}
        style={{
          position: "absolute",
          inset: 0,
          backgroundColor: appearance.handBgColor,
          backgroundImage: appearance.handTextureCss,
          transform: textureTransform,
          transformOrigin: "center center",
          pointerEvents: "none",
        }}
      />
      {resolvedPieceOrder.map((pieceType, index) => {
        const count = countByPieceType.get(pieceType) ?? 0;
        const isSelected = selectedPieceType === pieceType;
        const left = paddingX + pieceLefts[index];
        const top = getHandPieceTop(
          pieceType,
          resolvedDisplayColor,
          resolvedSeatPosition,
          pieceSize,
          scale,
        );

        if (count === 0) {
          return (
            <div
              key={pieceType}
              data-testid={`hand-slot-${color}-${pieceType}`}
              style={{
                position: "absolute",
                left: `${left}px`,
                top: `${top}px`,
                width: `${pieceSize}px`,
                height: `${pieceSize}px`,
                flexShrink: 0,
                zIndex: 1,
              }}
            />
          );
        }

        return (
          <button
            key={pieceType}
            type="button"
            data-testid={`hand-piece-${color}-${pieceType}`}
            aria-label={`${PIECE_KANJI[pieceType] ?? pieceType}${count > 1 ? count : ""}`}
            title={`${PIECE_KANJI[pieceType] ?? pieceType}${count > 1 ? count : ""}`}
            data-selected={isSelected ? "true" : "false"}
            onClick={(event) => {
              event.stopPropagation();
              onPieceClick(pieceType);
            }}
            onPointerDown={(event) => {
              event.stopPropagation();
              onPiecePointerDown(pieceType, event);
            }}
            onPointerUp={(event) => {
              event.stopPropagation();
              onPiecePointerUp?.(pieceType, event);
            }}
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              left: `${left}px`,
              top: `${top}px`,
              width: `${pieceSize}px`,
              height: `${pieceSize}px`,
              padding: 0,
              border: "none",
              background: isSelected ? "rgba(255, 255, 255, 0.26)" : "transparent",
              boxShadow: isSelected ? "inset 0 0 0 1px rgba(74, 53, 32, 0.3)" : "none",
              borderRadius: flatStyle ? "0" : `${Math.max(3, 4 * scale)}px`,
              cursor: "pointer",
              touchAction: "none",
              flexShrink: 0,
              position: "absolute",
              zIndex: 1,
            }}
          >
            <img
              src={getPieceImagePath(
                pieceType,
                color,
                false,
                resolvedDisplayColor !== color,
                pieceStyle,
              )}
              alt=""
              draggable={false}
              style={{
                width: `${pieceSize}px`,
                height: `${pieceSize}px`,
                display: "block",
                pointerEvents: "none",
                userSelect: "none",
              }}
            />
            {count > 1 && (
              <span
                data-testid={`hand-count-${color}-${pieceType}`}
                aria-hidden="true"
                style={{
                  position: "absolute",
                  ...getHandCountPosition(pieceType, resolvedDisplayColor, pieceSize, scale),
                  display: "inline-flex",
                  alignItems: resolvedDisplayColor === "white" ? "flex-start" : "flex-end",
                  justifyContent: "center",
                  transform: getHandCountOffsetTransform(resolvedDisplayColor, scale),
                  pointerEvents: "none",
                  zIndex: 1,
                }}
              >
                <span
                  style={{
                    ...getHandCountTextStyle(scale),
                    ...getHandCountTextTransform(resolvedDisplayColor),
                  }}
                >
                  {count}
                </span>
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

function getHandCountPosition(
  pieceType: string,
  displayColor: ColorData,
  pieceSize: number,
  scale: number,
): React.CSSProperties {
  if (displayColor === "white") {
    const bounds = getHandPieceBounds(pieceType, displayColor);
    return {
      left: toPx(getHandCountInset(bounds.left, HAND_IMAGE_SOURCE_WIDTH, pieceSize, scale)),
      top: toPx(getHandCountInset(bounds.top, HAND_IMAGE_SOURCE_HEIGHT, pieceSize, scale)),
    };
  }

  const bounds = getHandPieceBounds(pieceType, displayColor);

  return {
    right: toPx(getHandCountInset(bounds.right, HAND_IMAGE_SOURCE_WIDTH, pieceSize, scale)),
    bottom: toPx(getHandCountInset(bounds.bottom, HAND_IMAGE_SOURCE_HEIGHT, pieceSize, scale)),
  };
}

function getHandCountInset(
  sourceMargin: number,
  sourceExtent: number,
  pieceSize: number,
  scale: number,
): number {
  return Math.max(0, (sourceMargin / sourceExtent) * pieceSize - HAND_COUNT_CORNER_NUDGE * scale);
}

function toPx(value: number): string {
  return `${Math.round(value * 10) / 10}px`;
}

function getHandCountTextStyle(scale: number): React.CSSProperties {
  return {
    display: "block",
    fontSize: `${Math.max(11, 15 * scale)}px`,
    lineHeight: 1,
    fontWeight: 800,
    color: "#16110c",
    textShadow:
      "0 0 1px rgba(255, 247, 229, 0.98), 0 1px 2px rgba(255, 247, 229, 0.95), 0 0 4px rgba(255, 247, 229, 0.72)",
    fontVariantNumeric: "tabular-nums",
    fontFamily: '"Segoe UI", "Yu Gothic UI", sans-serif',
    textRendering: "geometricPrecision",
    WebkitFontSmoothing: "antialiased",
  };
}

function getHandCountTextTransform(displayColor: ColorData): React.CSSProperties {
  if (displayColor === "white") {
    return {
      transform: "rotate(180deg)",
      transformOrigin: "center center",
    };
  }

  return {};
}

function getHandCountOffsetTransform(displayColor: ColorData, scale: number): string {
  if (displayColor === "white") {
    return `translate(${toPx(-HAND_COUNT_OFFSET_X * scale)}, ${toPx(-HAND_COUNT_OFFSET_Y * scale)})`;
  }

  return `translateX(${toPx(HAND_COUNT_OFFSET_X * scale)})`;
}
