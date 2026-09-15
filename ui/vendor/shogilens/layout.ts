import { BOARD_SVG_HEIGHT, BOARD_SVG_WIDTH, CELL_SIZE } from "./constants";
import { HAND_PANEL_VERTICAL_PADDING } from "./handLayout";

const HAND_ROW_HEIGHT = CELL_SIZE + HAND_PANEL_VERTICAL_PADDING * 2;
const BOARD_HAND_GAP = 0;

export interface BoardLayoutMetrics {
  boardHandGap: number;
  frameHeight: number;
  frameWidth: number;
  handRowHeight: number;
  playerInfoHandGap: number;
  playerInfoRowHeight: number;
  seatHeight: number;
  showPlayerInfo: boolean;
  svgHeight: number;
  svgWidth: number;
}

/**
 * 盤面 UI の寸法を 1 か所で管理する。
 * 対局時のプレイヤー情報は持ち駒段の空き側に重ねるため、追加の縦段は持たない。
 */
export function getBoardLayoutMetrics(): BoardLayoutMetrics {
  const showPlayerInfo = true;
  const playerInfoRowHeight = 0;
  const playerInfoHandGap = 0;
  const handRowHeight = HAND_ROW_HEIGHT;
  const seatHeight = handRowHeight;
  const svgWidth = BOARD_SVG_WIDTH;
  const svgHeight = BOARD_SVG_HEIGHT;
  const frameWidth = BOARD_SVG_WIDTH;
  const frameHeight = seatHeight * 2 + BOARD_HAND_GAP * 2 + BOARD_SVG_HEIGHT;

  return {
    boardHandGap: BOARD_HAND_GAP,
    frameHeight,
    frameWidth,
    handRowHeight,
    playerInfoHandGap,
    playerInfoRowHeight,
    seatHeight,
    showPlayerInfo,
    svgHeight,
    svgWidth,
  };
}
