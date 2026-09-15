import { useEffect, useMemo, useRef } from "react";

interface BoardStageFrameOverlayProps {
  bottomHandSectionHeight: number;
  bottomAccentColor: string;
  fixedSectionWidth: number;
  frameColor: string;
  height: number;
  horizontalThickness: number;
  topAccentColor: string;
  topHandSectionHeight: number;
  verticalThickness: number;
  width: number;
}

function clampLength(value: number): number {
  return Math.max(0, value);
}

function toDevicePixels(value: number, devicePixelRatio: number): number {
  return Math.max(0, Math.round(value * devicePixelRatio));
}

/**
 * 盤全体の最外周フレームを描く。
 * device pixel 単位で描くことで、サイズ変更時のラスタ揺れを抑える。
 */
export function BoardStageFrameOverlay({
  bottomHandSectionHeight,
  bottomAccentColor,
  fixedSectionWidth,
  frameColor,
  height,
  horizontalThickness,
  topAccentColor,
  topHandSectionHeight,
  verticalThickness,
  width,
}: BoardStageFrameOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const snappedWidth = clampLength(width);
  const snappedHeight = clampLength(height);
  const devicePixelRatio = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;

  const metrics = useMemo(() => {
    const deviceWidth = toDevicePixels(snappedWidth, devicePixelRatio);
    const deviceHeight = toDevicePixels(snappedHeight, devicePixelRatio);
    const horizontalDeviceThickness = toDevicePixels(horizontalThickness, devicePixelRatio);
    const verticalDeviceThickness = toDevicePixels(verticalThickness, devicePixelRatio);
    const fixedSectionWidthDevice = Math.min(
      deviceWidth,
      toDevicePixels(fixedSectionWidth, devicePixelRatio),
    );
    const topHandSectionHeightDevice = Math.min(
      deviceHeight,
      toDevicePixels(topHandSectionHeight, devicePixelRatio),
    );
    const bottomHandSectionHeightDevice = Math.min(
      deviceHeight,
      toDevicePixels(bottomHandSectionHeight, devicePixelRatio),
    );

    return {
      bottomHandSectionHeightDevice,
      deviceWidth,
      deviceHeight,
      dynamicWidthDevice: clampLength(deviceWidth - fixedSectionWidthDevice),
      fixedSectionWidthDevice,
      horizontalDeviceThickness,
      topHandSectionHeightDevice,
      verticalDeviceThickness,
    };
  }, [
    bottomHandSectionHeight,
    devicePixelRatio,
    fixedSectionWidth,
    horizontalThickness,
    snappedHeight,
    snappedWidth,
    topHandSectionHeight,
    verticalThickness,
  ]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }

    canvas.width = metrics.deviceWidth;
    canvas.height = metrics.deviceHeight;

    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }

    context.clearRect(0, 0, metrics.deviceWidth, metrics.deviceHeight);
    context.fillStyle = frameColor;
    context.fillRect(0, 0, metrics.dynamicWidthDevice, metrics.horizontalDeviceThickness);
    context.fillRect(
      metrics.fixedSectionWidthDevice,
      metrics.deviceHeight - metrics.horizontalDeviceThickness,
      metrics.dynamicWidthDevice,
      metrics.horizontalDeviceThickness,
    );
    context.fillRect(
      0,
      0,
      metrics.verticalDeviceThickness,
      clampLength(metrics.deviceHeight - metrics.bottomHandSectionHeightDevice),
    );
    context.fillRect(
      metrics.deviceWidth - metrics.verticalDeviceThickness,
      metrics.topHandSectionHeightDevice,
      metrics.verticalDeviceThickness,
      clampLength(metrics.deviceHeight - metrics.topHandSectionHeightDevice),
    );

    context.fillStyle = topAccentColor;
    context.fillRect(
      metrics.deviceWidth - metrics.fixedSectionWidthDevice,
      0,
      metrics.fixedSectionWidthDevice,
      metrics.horizontalDeviceThickness,
    );
    context.fillRect(
      metrics.deviceWidth - metrics.verticalDeviceThickness,
      0,
      metrics.verticalDeviceThickness,
      metrics.topHandSectionHeightDevice,
    );

    context.fillStyle = bottomAccentColor;
    context.fillRect(
      0,
      metrics.deviceHeight - metrics.horizontalDeviceThickness,
      metrics.fixedSectionWidthDevice,
      metrics.horizontalDeviceThickness,
    );
    context.fillRect(
      0,
      metrics.deviceHeight - metrics.bottomHandSectionHeightDevice,
      metrics.verticalDeviceThickness,
      metrics.bottomHandSectionHeightDevice,
    );
  }, [bottomAccentColor, frameColor, metrics, topAccentColor]);

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-bottom-hand-section-device-height={String(metrics.bottomHandSectionHeightDevice)}
      data-horizontal-device-thickness={String(metrics.horizontalDeviceThickness)}
      data-top-hand-section-device-height={String(metrics.topHandSectionHeightDevice)}
      data-vertical-device-thickness={String(metrics.verticalDeviceThickness)}
      style={{
        display: "block",
        width: `${snappedWidth}px`,
        height: `${snappedHeight}px`,
      }}
    />
  );
}
