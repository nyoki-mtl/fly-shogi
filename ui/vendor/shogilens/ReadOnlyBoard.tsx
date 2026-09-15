import { useRef } from "react";
import type { BoardStyle, PieceStyle } from "../../lib/ipc/appSettings";
import { BoardSeatPanel } from "./BoardSeatPanel";
import { resolveBoardPlayerName } from "./boardPlayerName";
import { BoardCoordinateLabels, BoardGrid, BoardSurfaceBackground } from "./BoardSvgDecorations";
import { CELL_SIZE, getBoardAppearance, getBoardGeometry } from "./constants";
import { getBoardLayoutMetrics } from "./layout";
import type { PeekBoardReplayData } from "./PeekBoardPopover";
import { Square } from "./Square";
import type { ColorData, PieceData, PositionData, SquareData } from "./types";

interface ReadOnlyBoardProps {
  position: PositionData;
  lastMove: { from: SquareData | null; to: SquareData | null };
  isFlipped: boolean;
  pieceStyle: PieceStyle;
  boardStyle: BoardStyle;
  showCoordinates: boolean;
  scale: number;
  snappedSectionHeights?: {
    bottomSeat: number;
    frame: number;
    stage: number;
    topSeat: number;
  };
  replay?: PeekBoardReplayData | null;
  selected?: string | null;
  candidates?: string[];
  onSquareClick?: (square: string) => void;
  onHandClick?: (piece: string, color: ColorData) => void;
  disabled?: boolean;
}

const NOOP = (): void => undefined;

const NOOP_PIECE_POINTER = (_pieceType: string, _event: unknown): void => undefined;

function findPiece(board: PositionData["board"], file: number, rank: number): PieceData | null {
  const found = board.find((bp) => bp.square.file === file && bp.square.rank === rank);
  return found?.piece ?? null;
}

/**
 * view-only board renderer。
 * main board と同等の見た目（持ち駒・対局者名・時計・座標ラベル含む）を描画するが、
 * input / store mutation / interactive 機能は一切持たない。
 * hover overlay 用に設計されている。
 */
export function ReadOnlyBoard({
  position,
  lastMove,
  isFlipped,
  pieceStyle,
  boardStyle,
  showCoordinates,
  scale,
  snappedSectionHeights,
  replay = null, selected = null, candidates = [], onSquareClick, onHandClick, disabled = false,
}: ReadOnlyBoardProps) {
  const topHandPanelRef = useRef<HTMLDivElement>(null);
  const bottomHandPanelRef = useRef<HTMLDivElement>(null);
  const layoutMetrics = getBoardLayoutMetrics();
  const appearance = getBoardAppearance(boardStyle);
  const geometry = getBoardGeometry(isFlipped, showCoordinates);

  const svgWidth = layoutMetrics.svgWidth;
  const svgHeight = layoutMetrics.svgHeight;
  const renderedSvgWidth = svgWidth * scale;
  const renderedSvgHeight = snappedSectionHeights?.stage ?? svgHeight * scale;

  const topColor: ColorData = isFlipped ? "black" : "white";
  const bottomColor: ColorData = isFlipped ? "white" : "black";
  const topPlayerName = resolveBoardPlayerName(topColor, replay?.metadata ?? null, "", "");
  const bottomPlayerName = resolveBoardPlayerName(bottomColor, replay?.metadata ?? null, "", "");
  const topHand = topColor === "black" ? position.black_hand : position.white_hand;
  const bottomHand = bottomColor === "black" ? position.black_hand : position.white_hand;
  const sectionGap = `${layoutMetrics.boardHandGap * scale}px`;
  const frameWidth = `${layoutMetrics.frameWidth * scale}px`;
  const frameHeight = `${snappedSectionHeights?.frame ?? layoutMetrics.frameHeight * scale}px`;

  return (
    <div
      data-testid="read-only-board"
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "stretch",
        width: frameWidth,
        height: frameHeight,
        position: "relative",
        pointerEvents: disabled ? "none" : "auto",
      }}
    >
      <BoardSeatPanel
        handPanelRef={topHandPanelRef}
        hand={topHand}
        color={topColor}
        playerName={topPlayerName}
        isActive={position.turn === topColor}
        pieceStyle={pieceStyle}
        boardStyle={boardStyle}
        selectedPieceType={position.turn === topColor && selected?.endsWith("*") ? selected[0] : null}
        onPanelClick={NOOP}
        onPieceClick={(piece) => onHandClick?.(piece, topColor)}
        onPiecePointerDown={NOOP_PIECE_POINTER}
        showClock={layoutMetrics.showPlayerInfo}
        seatPosition="top"
        scale={scale}
        rowHeightOverride={snappedSectionHeights?.topSeat}
        renderClock={() => null}
      />

      <div
        style={{
          position: "relative",
          width: `${renderedSvgWidth}px`,
          height: `${renderedSvgHeight}px`,
          marginTop: sectionGap,
          marginBottom: sectionGap,
        }}
      >
        <svg
          width={renderedSvgWidth}
          height={renderedSvgHeight}
          viewBox={`0 0 ${svgWidth} ${svgHeight}`}
          aria-hidden="true"
          style={{ display: "block" }}
        >
          <BoardSurfaceBackground
            appearance={appearance}
            svgWidth={svgWidth}
            svgHeight={svgHeight}
          />
          <BoardGrid appearance={appearance} geometry={geometry} />

          {[
            [3, 3],
            [3, 6],
            [6, 3],
            [6, 6],
          ].map(([col, row]) => (
            <circle
              key={`star-${col}-${row}`}
              cx={geometry.paddingLeft + col * CELL_SIZE}
              cy={geometry.paddingTop + row * CELL_SIZE}
              r={3}
              fill={appearance.boardLine}
            />
          ))}

          <BoardCoordinateLabels
            appearance={appearance}
            geometry={geometry}
            isFlipped={isFlipped}
            visible={showCoordinates}
          />

          {Array.from({ length: 9 }, (_, col) =>
            Array.from({ length: 9 }, (_, row) => {
              const file = isFlipped ? col + 1 : 9 - col;
              const rank = isFlipped ? 9 - row : row + 1;
              const x = geometry.paddingLeft + col * CELL_SIZE;
              const y = geometry.paddingTop + row * CELL_SIZE;
              const piece = findPiece(position.board, file, rank);
              const isLastMoveSquare =
                (lastMove.from?.file === file && lastMove.from.rank === rank) ||
                (lastMove.to?.file === file && lastMove.to.rank === rank);

              return (
                <Square
                  key={`${file}-${rank}`}
                  x={x}
                  y={y}
                  file={file}
                  rank={rank}
                  isFlipped={isFlipped}
                  pieceStyle={pieceStyle}
                  piece={piece}
                  isSelected={selected === `${file}${"abcdefghi"[rank-1]}`}
                  isCandidate={candidates.includes(`${file}${"abcdefghi"[rank-1]}`)}
                  isLastMove={isLastMoveSquare}
                  isDragSource={false}
                  onClick={() => onSquareClick?.(`${file}${"abcdefghi"[rank-1]}`)}
                  onPointerDown={NOOP}
                />
              );
            }),
          )}
        </svg>
      </div>

      <BoardSeatPanel
        handPanelRef={bottomHandPanelRef}
        hand={bottomHand}
        color={bottomColor}
        playerName={bottomPlayerName}
        isActive={position.turn === bottomColor}
        pieceStyle={pieceStyle}
        boardStyle={boardStyle}
        selectedPieceType={position.turn === bottomColor && selected?.endsWith("*") ? selected[0] : null}
        onPanelClick={NOOP}
        onPieceClick={(piece) => onHandClick?.(piece, bottomColor)}
        onPiecePointerDown={NOOP_PIECE_POINTER}
        showClock={layoutMetrics.showPlayerInfo}
        seatPosition="bottom"
        scale={scale}
        rowHeightOverride={snappedSectionHeights?.bottomSeat}
        renderClock={() => null}
      />
    </div>
  );
}
