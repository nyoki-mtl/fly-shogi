import { BOARD_PADDING_LEFT, BOARD_PADDING_RIGHT, BOARD_SIZE } from "./constants";
import { getHandPanelHeight, getHandVisibleGap, getStandardHandPanelWidth } from "./handLayout";

const PLAYER_INFO_CARD_MAX_WIDTH = 196;
const PLAYER_INFO_CARD_MAX_WIDTH_SCALED = 176;

export interface SeatFrameMetrics {
  accentThickness: number;
  cardWidth: number;
  fullBoardWidth: number;
  handRowHeight: number;
}

export function getSeatFrameMetrics(
  scale: number,
  fullBoardWidthOverride?: number,
): SeatFrameMetrics {
  const fullBoardWidth =
    fullBoardWidthOverride ?? (BOARD_PADDING_LEFT + BOARD_SIZE + BOARD_PADDING_RIGHT) * scale;
  const handWidth = getStandardHandPanelWidth(scale, getHandVisibleGap(scale));
  const playerInfoGutterWidth = Math.max(0, fullBoardWidth - handWidth);
  const cardMaxWidth = Math.min(
    PLAYER_INFO_CARD_MAX_WIDTH,
    PLAYER_INFO_CARD_MAX_WIDTH_SCALED * scale,
  );
  const cardWidth = Math.min(playerInfoGutterWidth, cardMaxWidth);
  const handRowHeight = getHandPanelHeight(scale);
  const accentThickness = Math.max(2, Math.min(3, 2.5 * scale));

  return {
    accentThickness,
    cardWidth,
    fullBoardWidth,
    handRowHeight,
  };
}
