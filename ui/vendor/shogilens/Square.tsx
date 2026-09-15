import type React from "react";
import type { PieceStyle } from "../../lib/ipc/appSettings";
import { CELL_SIZE, COLORS } from "./constants";
import { Piece } from "./Piece";
import type { PieceData } from "./types";

interface SquareProps {
  x: number;
  y: number;
  file: number;
  rank: number;
  isFlipped: boolean;
  pieceStyle: PieceStyle;
  piece: PieceData | null;
  isSelected: boolean;
  isCandidate: boolean;
  isLastMove: boolean;
  isDragSource: boolean;
  onClick: () => void;
  onDoubleClick?: () => void;
  onContextMenu?: (e: React.MouseEvent<SVGGElement>) => void;
  onPointerDown: (e: React.PointerEvent) => void;
  onPointerUp?: (e: React.PointerEvent) => void;
}

/** マス目コンポーネント */
export function Square({
  x,
  y,
  file,
  rank,
  isFlipped,
  pieceStyle,
  piece,
  isSelected,
  isCandidate,
  isLastMove,
  isDragSource,
  onClick,
  onDoubleClick,
  onContextMenu,
  onPointerDown,
  onPointerUp,
}: SquareProps) {
  let fillColor = "transparent";
  if (isSelected) {
    fillColor = COLORS.selectedSquare;
  } else if (isCandidate) {
    fillColor = COLORS.candidateSquare;
  } else if (isLastMove) {
    fillColor = COLORS.lastMoveSquare;
  }

  return (
    <g
      data-testid={`square-${file}-${rank}`}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
      style={{ cursor: "pointer" }}
    >
      {/* ヒットエリア（常にレンダリングしてクリック領域を確保） */}
      <rect
        x={x}
        y={y}
        width={CELL_SIZE}
        height={CELL_SIZE}
        fill={fillColor}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
      />
      {/* 移動先候補のドット表示（駒がないマス） */}
      {isCandidate && !piece && (
        <circle
          cx={x + CELL_SIZE / 2}
          cy={y + CELL_SIZE / 2}
          r={CELL_SIZE * 0.15}
          fill="rgba(70, 180, 70, 0.6)"
        />
      )}
      {/* 駒の描画（ドラッグ中は非表示） */}
      {piece && !isDragSource && (
        <Piece piece={piece} x={x} y={y} isFlipped={isFlipped} pieceStyle={pieceStyle} />
      )}
    </g>
  );
}
