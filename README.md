# Dōbutsu Shōgi

A web-based playable implementation of [Dōbutsu shōgi](https://en.wikipedia.org/wiki/D%C5%8Dbutsu_sh%C5%8Dgi) — the 3 × 4 Animal Shogi variant invented in 2008 by Madoka Kitao. Game logic and AI are written in Rust, compiled to WebAssembly; the UI is plain TypeScript. Everything runs client-side.

## Status

Pre-alpha. Spec stable, implementation in progress.

## Live demo

TBA — published via GitHub Pages on every push to `main`.

## Local development

Prerequisites: Rust (stable, ≥ 1.85 for edition 2024), Node 20, [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/).

```sh
# native tests for the rules + AI crate
cargo test

# build the wasm module (outputs to web/pkg/)
wasm-pack build --target web --out-dir web/pkg

# install web deps and start the dev server
cd web && npm install && npm run dev
```

For a production build, add `--release` to `wasm-pack` and run `npm run build` in `web/` (outputs `web/dist/`).

## Architecture

```
+--------------------------+
|  TypeScript UI  (web/)   |  click-click + drag, SVG board, animations
+--------------------------+
              |
              | u8 move codes / Uint8Array board
              v
+--------------------------+
|  Rust core   (src/)      |  rules + search, exposed via wasm-bindgen
+--------------------------+
```

The Rust crate is the source of truth for game rules and AI. It builds natively (for `cargo test` and any future CLI / TUI / solver) and to `wasm32-unknown-unknown` (for the browser). The TypeScript layer never re-implements rules — it only renders state and forwards moves.

See [SPEC.md](SPEC.md) for the full design.

## Ideas

Deferred from v1; tracked here so they don't get lost.

- Stronger / human-like AI (the eventual goal — the v1 engine is intentionally weak).
- Repetition draws (~2.7 % of reachable non-terminal positions per Tanaka 2009; v1 ignores them).
- Eval / debug overlay (toggle via `?d=1` URL param: shows engine score, principal variation, search stats).
- Move-history sidebar with click-to-jump.
- Save / load game (localStorage; share-by-URL position encoding).
- Touch drag-and-drop (v1 uses click-click on touch; HTML5 DnD on pointer only).
- Adjustable AI depth and animation speed.
- Run the search in a Web Worker.
- Position editor.
- Keyboard navigation and screen-reader support.
- Alternative piece glyphs (animal SVGs, kanji).
- Native TUI front-end against the same crate.

## License

MIT.
