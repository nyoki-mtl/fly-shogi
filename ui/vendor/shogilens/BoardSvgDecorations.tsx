import type { BoardAppearance, BoardGeometry } from "./constants";
import { BOARD_SIZE, CELL_SIZE, RANK_KANJI } from "./constants";

const BOARD_SURFACE_RADIUS = 0;
const COORDINATE_FONT_FAMILY = '"Yu Mincho", "Hiragino Mincho ProN", "Noto Serif JP", serif';
const COORDINATE_FONT_SIZE = 10;
const COORDINATE_FONT_WEIGHT = 700;
const BOARD_OUTLINE_RADIUS = 0;

interface BoardGridProps {
  appearance: BoardAppearance;
  geometry: BoardGeometry;
}

interface BoardSurfaceBackgroundProps {
  appearance: BoardAppearance;
  svgHeight: number;
  svgWidth: number;
}

interface BoardCoordinateLabelsProps {
  appearance: BoardAppearance;
  geometry: BoardGeometry;
  isFlipped: boolean;
  visible: boolean;
}

/** 盤面全体の木地を描く。盤の外周に薄い囲みを作るため、余白も同じテクスチャで埋める。 */
export function BoardSurfaceBackground({
  appearance,
  svgHeight,
  svgWidth,
}: BoardSurfaceBackgroundProps) {
  return (
    <>
      <rect
        x={0}
        y={0}
        width={svgWidth}
        height={svgHeight}
        rx={BOARD_SURFACE_RADIUS}
        ry={BOARD_SURFACE_RADIUS}
        fill={appearance.boardBg}
      />
      <foreignObject x={0} y={0} width={svgWidth} height={svgHeight}>
        <div
          style={{
            width: "100%",
            height: "100%",
            borderRadius: `${BOARD_SURFACE_RADIUS}px`,
            background: appearance.boardTextureCss,
          }}
        />
      </foreignObject>
    </>
  );
}

/** 盤面の罫線と外周枠を描く。外周は rect にして角のつながりを安定させる。 */
export function BoardGrid({ appearance, geometry }: BoardGridProps) {
  return (
    <>
      {Array.from({ length: 8 }, (_, index) => {
        const offset = CELL_SIZE * (index + 1);
        return (
          <g key={`grid-${String(index)}`}>
            <line
              x1={geometry.paddingLeft + offset}
              y1={geometry.paddingTop}
              x2={geometry.paddingLeft + offset}
              y2={geometry.paddingTop + BOARD_SIZE}
              stroke={appearance.boardLine}
              strokeWidth={1}
            />
            <line
              x1={geometry.paddingLeft}
              y1={geometry.paddingTop + offset}
              x2={geometry.paddingLeft + BOARD_SIZE}
              y2={geometry.paddingTop + offset}
              stroke={appearance.boardLine}
              strokeWidth={1}
            />
          </g>
        );
      })}

      <rect
        x={geometry.paddingLeft}
        y={geometry.paddingTop}
        width={BOARD_SIZE}
        height={BOARD_SIZE}
        rx={BOARD_OUTLINE_RADIUS}
        ry={BOARD_OUTLINE_RADIUS}
        fill="none"
        stroke={appearance.boardLine}
        strokeWidth={appearance.boardOutlineWidth}
        strokeLinejoin="round"
      />
    </>
  );
}

/** 盤外の筋・段ラベルを描く。 */
export function BoardCoordinateLabels({
  appearance,
  geometry,
  isFlipped,
  visible,
}: BoardCoordinateLabelsProps) {
  if (!visible) {
    return null;
  }

  const fileLabelY = isFlipped ? geometry.fileLabelBottomCenterY : geometry.fileLabelTopCenterY;
  const rankLabelX = isFlipped ? geometry.rankLabelLeftCenterX : geometry.rankLabelRightCenterX;

  return (
    <>
      {Array.from({ length: 9 }, (_, col) => {
        const file = isFlipped ? col + 1 : 9 - col;
        const x = geometry.paddingLeft + col * CELL_SIZE + CELL_SIZE / 2;
        return (
          <text
            key={`file-label-${String(file)}`}
            x={x}
            y={fileLabelY}
            fill={appearance.coordLabel}
            fontFamily={COORDINATE_FONT_FAMILY}
            fontSize={COORDINATE_FONT_SIZE}
            fontWeight={COORDINATE_FONT_WEIGHT}
            textAnchor="middle"
            dominantBaseline="middle"
            transform={isFlipped ? `rotate(180 ${String(x)} ${String(fileLabelY)})` : undefined}
            aria-hidden="true"
          >
            {String(file)}
          </text>
        );
      })}

      {Array.from({ length: 9 }, (_, row) => {
        const rank = isFlipped ? 9 - row : row + 1;
        const y = geometry.paddingTop + row * CELL_SIZE + CELL_SIZE / 2;
        return (
          <text
            key={`rank-label-${String(rank)}`}
            x={rankLabelX}
            y={y}
            fill={appearance.coordLabel}
            fontFamily={COORDINATE_FONT_FAMILY}
            fontSize={COORDINATE_FONT_SIZE}
            fontWeight={COORDINATE_FONT_WEIGHT}
            textAnchor="middle"
            dominantBaseline="middle"
            transform={isFlipped ? `rotate(180 ${String(rankLabelX)} ${String(y)})` : undefined}
            aria-hidden="true"
          >
            {RANK_KANJI[rank]}
          </text>
        );
      })}
    </>
  );
}
