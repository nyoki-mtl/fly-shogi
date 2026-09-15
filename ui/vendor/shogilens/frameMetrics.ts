const OUTER_FRAME_BASE_THICKNESS_PX = 4;
const OUTER_FRAME_LABEL_CLEARANCE_PX = 1;

export const HIGHLIGHT_FRAME_COLOR = "rgba(47, 109, 181, 0.95)";

export function snapToDevicePixel(value: number, devicePixelRatio: number): number {
  return Math.round(value * devicePixelRatio) / devicePixelRatio;
}

function getStableFrameThickness(baseThickness: number, devicePixelRatio: number): number {
  if (Number.isInteger(devicePixelRatio)) {
    return baseThickness;
  }

  // Fractional DPI tends to rasterize thick crisp-edge fills heavier than their CSS px value.
  return Math.max(1 / devicePixelRatio, baseThickness - 1 / devicePixelRatio);
}

export function getOuterFrameMetrics(devicePixelRatio: number): {
  frameOutset: number;
  frameThickness: number;
} {
  const frameThickness = snapToDevicePixel(
    getStableFrameThickness(OUTER_FRAME_BASE_THICKNESS_PX, devicePixelRatio),
    devicePixelRatio,
  );
  const frameOutset = snapToDevicePixel(
    frameThickness + OUTER_FRAME_LABEL_CLEARANCE_PX,
    devicePixelRatio,
  );

  return {
    frameOutset,
    frameThickness,
  };
}
