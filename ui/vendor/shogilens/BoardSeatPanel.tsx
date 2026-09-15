import type { PointerEvent, ReactNode, RefObject } from "react";
import type { BoardStyle, PieceStyle } from "../../lib/ipc/appSettings";
import { HAND_PIECE_ORDER } from "./constants";
import { Hand } from "./Hand";
import { getSeatCardPalette } from "./seatCardPalette";
import { getSeatFrameMetrics } from "./seatFrameMetrics";
import type { ColorData, PositionData } from "./types";

interface BoardSeatPanelProps {
  handPanelRef: RefObject<HTMLDivElement | null>;
  hand: PositionData["black_hand"];
  color: ColorData;
  playerName: string;
  isActive: boolean;
  pieceStyle: PieceStyle;
  boardStyle: BoardStyle;
  selectedPieceType: string | null;
  onPanelClick: () => void;
  onPieceClick: (pieceType: string) => void;
  onPiecePointerDown: (pieceType: string, event: PointerEvent) => void;
  onPiecePointerUp?: (pieceType: string, event: PointerEvent) => void;
  showClock: boolean;
  seatPosition: "top" | "bottom";
  scale: number;
  rowHeightOverride?: number;
  fullBoardWidthOverride?: number;
  renderClock?: (args: { color: ColorData; fontSize: number; scale: number }) => ReactNode;
  suppressHandBorderTop?: boolean;
  suppressHandBorderBottom?: boolean;
}

export function BoardSeatPanel({
  handPanelRef,
  hand,
  color,
  playerName,
  isActive,
  pieceStyle,
  boardStyle,
  selectedPieceType,
  onPanelClick,
  onPieceClick,
  onPiecePointerDown,
  onPiecePointerUp,
  showClock,
  seatPosition,
  scale,
  rowHeightOverride,
  fullBoardWidthOverride,
  renderClock,
  suppressHandBorderTop = false,
  suppressHandBorderBottom = false,
}: BoardSeatPanelProps) {
  const {
    accentThickness: cardAccentThickness,
    cardWidth,
    fullBoardWidth,
    handRowHeight,
  } = getSeatFrameMetrics(scale, fullBoardWidthOverride);
  const cardHeight = rowHeightOverride ?? handRowHeight;
  const palette = getSeatCardPalette(color);
  const displayName = playerName.trim();
  const dividerColor = "rgba(58, 43, 26, 0.1)";
  const dividerWidth = 1;
  const innerGap = Math.max(4, Math.min(8, 5 * scale));
  const contentPaddingBlock = Math.max(1, Math.min(3, 2 * scale));
  const namePaddingInline = Math.max(6, Math.min(12, 8 * scale));
  const clockPaddingLeading = Math.max(6, Math.min(12, 8 * scale));
  const cardAccentInset = cardAccentThickness + innerGap;
  const namePaddingLeft = seatPosition === "bottom" ? cardAccentInset : namePaddingInline;
  const namePaddingRight = seatPosition === "top" ? cardAccentInset : namePaddingInline;
  const clockPaddingTrailing = cardAccentInset;
  const clockPaddingLeft = seatPosition === "bottom" ? clockPaddingTrailing : clockPaddingLeading;
  const clockPaddingRight = seatPosition === "top" ? clockPaddingTrailing : clockPaddingLeading;
  const { clockFontSize, nameFontSize: baseNameFontSize } = getSeatTypographyMetrics(
    cardWidth,
    cardHeight,
    scale,
  );
  const nameTextAvailableWidth = cardWidth - namePaddingLeft - namePaddingRight;
  const letterSpacingScale = cardWidth < 108 ? 1.02 : 1.04;
  const nameFontSize = Math.max(
    7,
    Math.min(baseNameFontSize, nameTextAvailableWidth / (7 * letterSpacingScale)),
  );
  const playerNameMaxChars = getPlayerNameMaxChars(cardWidth);
  const truncatedName = truncatePlayerName(displayName, /^[\x20-\x7e]+$/.test(displayName) ? playerNameMaxChars * 2 : playerNameMaxChars);
  const frameShadow = "none";

  const handRow = (
    <Hand
      panelRef={handPanelRef}
      hand={hand}
      color={color}
      align={seatPosition === "top" ? "start" : "end"}
      displayColor={seatPosition === "top" ? "white" : "black"}
      pieceOrder={seatPosition === "top" ? [...HAND_PIECE_ORDER].reverse() : HAND_PIECE_ORDER}
      seatPosition={seatPosition}
      pieceStyle={pieceStyle}
      boardStyle={boardStyle}
      selectedPieceType={selectedPieceType}
      onPanelClick={onPanelClick}
      onPieceClick={onPieceClick}
      onPiecePointerDown={onPiecePointerDown}
      onPiecePointerUp={onPiecePointerUp}
      scale={scale}
      flatStyle
      fullBoardWidthOverride={fullBoardWidthOverride}
      suppressBorderTop={suppressHandBorderTop}
      suppressBorderBottom={suppressHandBorderBottom}
    />
  );

  if (!showClock) {
    if (rowHeightOverride === undefined) {
      return handRow;
    }

    return (
      <div
        style={{
          position: "relative",
          width: `${fullBoardWidth}px`,
          height: `${rowHeightOverride}px`,
        }}
      >
        <div
          style={{
            position: "absolute",
            left: 0,
            [seatPosition === "top" ? "bottom" : "top"]: 0,
          }}
        >
          {handRow}
        </div>
      </div>
    );
  }

  const anchoredHandRow =
    rowHeightOverride === undefined ? (
      handRow
    ) : (
      <div
        style={{
          position: "absolute",
          left: 0,
          [seatPosition === "top" ? "bottom" : "top"]: 0,
        }}
      >
        {handRow}
      </div>
    );

  return (
    <div
      style={{
        position: "relative",
        width: `${fullBoardWidth}px`,
        height: `${rowHeightOverride ?? handRowHeight}px`,
        boxSizing: "border-box",
      }}
    >
      {anchoredHandRow}
      <div
        data-testid={`player-info-${color}`}
        data-active-turn={isActive ? "true" : "false"}
        style={{
          position: "absolute",
          [seatPosition === "top" ? "bottom" : "top"]: 0,
          [seatPosition === "top" ? "right" : "left"]: 0,
          transform: "none",
          width: `${cardWidth}px`,
          height: `${cardHeight}px`,
          overflow: "hidden",
          borderRadius: "0",
          border: "0 solid transparent",
          background: palette.background,
          boxShadow: frameShadow,
          boxSizing: "border-box",
        }}
      >
        <div
          data-testid={`player-info-content-${color}`}
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            display: "grid",
            gridTemplateRows: "1fr 1fr",
            zIndex: 0,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              borderBottom: `${dividerWidth}px solid ${dividerColor}`,
              background: "transparent",
              padding: `${contentPaddingBlock}px ${namePaddingRight}px ${contentPaddingBlock}px ${namePaddingLeft}px`,
            }}
          >
            <div
              data-testid={`player-name-${color}`}
              title={displayName}
              style={{
                minWidth: 0,
                width: "100%",
                fontSize: `${nameFontSize}px`,
                fontWeight: 800,
                color: palette.nameText,
                letterSpacing: cardWidth < 108 ? "0.02em" : "0.04em",
                textAlign: "center",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "clip",
              }}
            >
              {truncatedName}
            </div>
          </div>

          <div
            data-testid={`player-clock-row-${color}`}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: isActive ? palette.activeOverlay : "transparent",
              padding: `${contentPaddingBlock}px ${clockPaddingRight}px ${contentPaddingBlock}px ${clockPaddingLeft}px`,
            }}
          >
            <div style={{ minWidth: 0, width: "100%" }}>
              {renderClock ? (
                renderClock({ color, fontSize: clockFontSize, scale })
              ) : (
                null
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function getSeatTypographyMetrics(
  cardWidth: number,
  cardHeight: number,
  scale: number,
): {
  clockFontSize: number;
  nameFontSize: number;
} {
  const nameFontSize = Math.max(
    7,
    Math.min(13, cardHeight * 0.19, cardWidth * (scale < 0.8 ? 0.125 : 0.115)),
  );
  const clockFontSize = Math.max(
    10,
    Math.min(18, cardHeight * 0.28, cardWidth * (scale < 0.8 ? 0.18 : 0.165)),
  );

  return {
    clockFontSize,
    nameFontSize,
  };
}

function truncatePlayerName(playerName: string, maxChars: number): string {
  const chars = Array.from(playerName);
  return chars.length <= maxChars ? playerName : chars.slice(0, maxChars).join("");
}

function getPlayerNameMaxChars(cardWidth: number): number {
  if (cardWidth < 132) {
    return 7;
  }
  return 8;
}
