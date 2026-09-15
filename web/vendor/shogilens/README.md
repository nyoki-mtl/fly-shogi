# ShogiLens board components

Source: ShogiLens by Hiroki Taniai, revision `5b508c3b39fdad12ba8f6c0e1a74ade7a8fdb4f0`.
The [included board source](../../../ui/vendor/shogilens/) is available in this package.
Board source license: MIT (`LICENSE`).

`ui/vendor/shogilens/` contains the React board sources. `Hand`, `Piece`, `Square`, `BoardSvgDecorations`, `BoardStageFrameOverlay`, geometry, hand layout, palette, and frame metrics are copied unchanged. The 28 hitomoji images originate from [sunfish-shogi/shogi-images](https://github.com/sunfish-shogi/shogi-images) under CC0 1.0 Universal; see their [source and license](../../pieces/hitomoji/README.md).

`ReadOnlyBoard` adds host interaction/selection props and removes replay-clock computation. `BoardSeatPanel` removes its Tauri-backed default `Clock` and allows twice as many ASCII characters as Japanese characters, so “Fly Meijin” fits the existing name card. Its supplied `renderClock` slot is used instead. The visual structure and inline styles, including both hand panels and player cards, remain the original components. The host supplies player names and no clock value because this demo has no timed game session.

`ui/shogilens/entry.tsx` converts the Rust API snapshot into the original component props, provides an accessible input bridge, and mounts the original frame overlay.

Build: `npm ci` then `npm run build:board`. Type check: `npx tsc --noEmit`.
The browser bundle includes React and React DOM. Their licenses are included alongside this file.
