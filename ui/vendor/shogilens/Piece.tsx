import type { PieceStyle } from "../../lib/ipc/appSettings";
import { getPieceImagePath, CELL_SIZE } from "./constants";
import type { PieceData } from "./types";

interface PieceProps {
  piece: PieceData;
  x: number;
  y: number;
  isFlipped: boolean;
  pieceStyle: PieceStyle;
}

/** 駒コンポーネント（SVG image 要素） */
export function Piece({ piece, x, y, isFlipped, pieceStyle }: PieceProps) {
  const imagePath = getPieceImagePath(
    piece.piece_type,
    piece.color,
    piece.promoted,
    isFlipped,
    pieceStyle,
  );

  return (
    <image
      href={imagePath}
      x={x}
      y={y}
      width={CELL_SIZE}
      height={CELL_SIZE}
      style={{ pointerEvents: "none" }}
    />
  );
}
