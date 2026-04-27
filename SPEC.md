# Dōbutsu Shōgi — Specification

Design document for the v1 web app. References Tanaka 2009 ("どうぶつしょうぎ" の完全解析, see `docs/`) for rule details.

---

## 1. Game rules

The cup-tournament variant analysed by Tanaka 2009.

**Board.** 3 columns × 4 rows. Columns A/B/C left-to-right; rows 1/2/3/4 top-to-bottom (Sente at row 4, Gote at row 1, in the source notation). Internally we use `(row, col)` with row 0 = mover's back rank, row 3 = opponent's back rank ("Try" target).

**Pieces (each side starts with 4).**

| Piece    | Moves                                                            |
|----------|------------------------------------------------------------------|
| Lion     | 8 surrounding cells (king).                                      |
| Giraffe  | 4 orthogonal neighbours.                                         |
| Elephant | 4 diagonal neighbours.                                           |
| Chick    | 1 cell forward. Promotes to **Hen** on reaching last rank.       |
| Hen      | 6 cells: 3 forward, 2 sideways, 1 backward (gold-general moves). |

**Captures.** Moving onto an enemy piece removes it from the board and adds it to the mover's hand. A captured Hen reverts to a Chick in hand.

**Drops.** On any empty square. No restrictions: Chick may be dropped on the last rank (where it cannot move), pawn-mate is allowed, perpetual check is allowed. (Confirmed by Tanaka 2009.)

**Win conditions.**

- **Lion capture**: the side that captures the opposing Lion wins.
- **Try**: the side whose Lion sits on the opponent's back rank *and is not under attack there* wins. The check fires (a) immediately after the move that places the Lion (eager Try) and (b) at the start of the next turn (Lion survived a full turn). Both branches are necessary because of the side-relative state encoding — see `rules.rs` for details.

**Stalemate / no legal moves.** Per Tanaka 2009: no-legal-moves positions exist on paper but are *unreachable* from the initial position via legal play. Defensive handling only: if `gen_moves` returns empty during search, treat as a loss for the side to move.

**Repetition.** Tournament rule: same position reached three times = draw. **Skipped in v1.** Listed in the README `Ideas` section.

---

## 2. Visual design

### 2.1 Pieces

Each piece is a pentagon — a square with the two front corners cut off. The cut depth is `side / 3`, removing 1⁄9 of the square's area. The pointed end faces the opponent; orientation alone signals which side owns the piece.

The piece's *interior* shows a 3 × 3 grid of dots indicating exactly which directions it can move from its current owner's perspective. This is both the identity glyph and a movement legend — no kanji, no animals, no letters.

Dot patterns (top of the diagram = forward, owner's perspective):

```
  Lion          Giraffe        Elephant       Chick           Hen
  • • •         · • ·          • · •          · • ·           • • •
  • · •         • · •          · · ·          · · ·           • · •
  • • •         · • ·          • · •          · · ·           · • ·
```

When a piece is captured and dropped by the other side, it is rotated 180° as a whole — pentagon and dots together. The dots therefore always describe legal moves *from the current owner's perspective*.

### 2.2 Piece colours

The pentagon outline and fill are neutral; the **dots** carry per-piece-type colour:

| Piece    | Dot colour       |
|----------|------------------|
| Lion     | amber `#d4a017`  |
| Giraffe  | orange `#e08a3c` |
| Elephant | slate `#5b8294`  |
| Chick    | yellow `#f4c542` |
| Hen      | coral `#d96666`  |

Colour means *piece type*, not *owner*. Owner is conveyed by pentagon orientation only.

### 2.3 Board

- 3 × 4 grid of cells, hairline lines, no shading.
- Always rendered from the **human player's** point of view: their back rank at the bottom of the screen, regardless of which side they picked.
- Promotion zone gets no special marking — promotion is automatic and visually obvious because Chick → Hen changes the dot pattern and colour.
- A **last-move highlight** (faint accent fill, opacity ~0.15) marks both the *from* and *to* squares after every move (human's and AI's).
- A **selection highlight** (thicker stroke + accent) marks the currently selected piece.
- Legal targets show a small **dot** on empty squares and a **ring** on enemy-occupied squares (capture).
- A **current-player indicator** (small accent dot) sits next to the active side's hand strip.

### 2.4 Hands

- AI's hand strip above the board, human's below.
- One pentagon **per captured piece** (not stacked with a count).
- Centred row, can grow wide. Maximum hand size in DS is small enough that wrap handling is unnecessary.
- Hand pentagons are oriented as if their owner were about to drop them.
- Empty hand = empty centred space (no placeholder slots).

### 2.5 Palette

| Role                                | Colour       |
|-------------------------------------|--------------|
| Background                          | `#fafaf7`    |
| Strokes (board, pentagon outline)   | `#2a2a2a`    |
| Accent (selection, last-move, dots) | `#5b8def`    |
| Disabled / loading                  | `#9a9a9a`    |
| Plus piece-type colours from §2.2.

One accent colour. Selection at full opacity, last-move at ~0.15, legal-target dots at ~0.5.

### 2.6 Layout

- Single column always: AI hand · board · human hand · controls (New game, Undo).
- Board sized `min(95vw, 480px)`. Pentagons scale with the cell.
- Works on phone and desktop without media queries.

---

## 3. Move input

**Click-click.**

- Click an own piece (board or hand) → it becomes selected, legal targets light up.
- Click a highlighted target → move.
- Click the selected piece again, or click anywhere illegal → deselect.
- Click another own piece → switch selection.

**Drag and drop** (desktop pointer only; click-click is the touch input).

- Drag from any own piece → same legal-target highlights appear.
- Drop on a highlighted target → move.
- Drop elsewhere → cancel; nothing remains selected.

**Esc** cancels selection at any time.

**Animations.**

- AI move: ~150 ms slide on the moving piece. Captured piece fades out then re-appears in the captor's hand. Implemented with CSS transforms on absolutely-positioned piece elements.
- Human move: instant on click; on drag, the piece is already at the destination when the drop fires.

---

## 4. AI / search

Intentionally weak in v1. The "good AI" goal is deferred — see README `Ideas`.

- **Algorithm**: negamax with alpha-beta pruning.
- **Depth**: fixed at 4 plies.
- **Move ordering**: captures before quiet moves, captures sorted by victim value (MVV).
- **Leaf evaluation** (from side-to-move's perspective):

  | Piece    | Value |
  |----------|-------|
  | Chick    | 1     |
  | Elephant | 5     |
  | Giraffe  | 5     |
  | Hen      | 6     |
  | Lion     | —     |

  Hand pieces count the same as on-board pieces. The Lion has no material value: it cannot be captured without ending the game.

- **Terminal scoring**: `LionCaptured` and `Try` return `±(MATE − ply_from_root)` so the engine prefers faster wins and slower losses. `MATE` is large (e.g. 10 000) and dominates any material differential.
- **Defensive**: empty `gen_moves` → return loss for STM.

**Thinking indicator.** A pulsing accent dot in the AI's hand strip while the engine searches, with a minimum visible duration of 250 ms so depth-4 (effectively instant) doesn't feel teleported.

---

## 5. Game lifecycle

**New-game modal.** Appears on first load and on every "New game" click. Two buttons: **Play first** / **Play second**. No Sente / Gote vocabulary. Selecting one starts the game; AI plays its first move immediately if "Play second" was chosen.

**Undo.** Button below the human's hand strip. Enabled only when (a) it is the human's turn and (b) the move history has ≥ 2 entries (so there is a previous human-decision state to return to). Clicking pops to the previous human-decision state in one click — that is, it undoes the AI's reply *and* the human's preceding move together. Without this, the AI would simply re-trigger and the user would be stuck.

Implementation: the JS layer keeps `Move[]` history. Undo = reset to initial position + replay all but the last *N* moves (N = 1 if only the human just moved, 2 if the AI has replied). Replay at depth ≤ ~80 plies takes microseconds; no wasm-side snapshot/restore API is needed.

No redo.

**End-of-game banner.** On terminal outcome (Lion captured, Try) display a banner with the result text and a "New game" button.

**Loading.** While the wasm module is fetching, the New-game modal's buttons are disabled and a small "loading…" caption is shown beneath them. No full-screen splash.

**Error fallback.** If the wasm module throws unexpectedly, replace the board area with a banner: "Something went wrong" + "New game" button.

---

## 6. Cross-boundary (Rust ⇄ TypeScript) API

The wasm module is built with `wasm-pack --target web` and exposes one opaque class.

```ts
class Game {
  constructor(humanPlaysFirst: boolean);
  legal_moves(): Uint8Array;          // packed move codes (see below)
  apply(move: number): Outcome;       // mutates self; advances to next side's turn
  board(): Uint8Array;                // 12 bytes, encoding piece+color per square
  hand_own(): { chick: number, elephant: number, giraffe: number };
  hand_opp(): { chick: number, elephant: number, giraffe: number };
  ai_move(depth: number): { mv: number, eval: number };
  free(): void;                       // wasm-bindgen-provided
}

enum Outcome { Ongoing, LionCaptured, Try }   // tsify-derived discriminated union
```

**Move encoding (single `u8`).**

- bits 0..3 (`to`): destination square 0..11 (`row * 3 + col`).
- bits 4..7 (`from`): source square 0..11 for slides; **codes 12 / 13 / 14** for drops of Chick / Elephant / Giraffe respectively.

A small `decodeMove(code: number): {kind: 'slide', from: number, to: number} | {kind: 'drop', piece: 'chick'|'elephant'|'giraffe', to: number}` helper lives on the JS side for rendering.

**Board encoding (12 bytes).** Each byte is the piece on that square, with colour. `0` = empty; otherwise low nibble = piece (1 Lion, 2 Giraffe, 3 Elephant, 4 Chick, 5 Hen), high bit = owner (0 = human's, 1 = AI's). Final encoding fixed at implementation time; this is the intent.

**Why `u8` move codes instead of a Tsify enum.** The JS side wants `Set<number>` legality lookups against drag/click targets. Comparing primitives is trivial; comparing tagged objects requires custom equality. The integer encoding is also smaller on the wire.

---

## 7. Repository layout

```
/Cargo.toml              workspace-less single crate
/src/
  lib.rs                 module re-exports
  rules.rs               game rules (committed)
  search.rs              negamax + alpha-beta + eval (TODO)
  wasm.rs                wasm-bindgen surface, gated by cfg(target_arch="wasm32")
/web/
  package.json
  vite.config.ts         base: './'
  tsconfig.json          strict
  index.html
  src/
    main.ts
    board.ts             SVG board renderer
    pieces.ts            pentagon + dot patterns
    input.ts             click-click + drag handling
    game.ts              wraps the wasm Game with undo history
  pkg/                   wasm-pack output, gitignored
/.github/workflows/
  deploy.yml             build + deploy to GitHub Pages on push to main
/docs/
  tanaka_animal_shogi_complete_analysis_jp.pdf   solver paper (committed)
/README.md
/SPEC.md                 this file
/LICENSE                 MIT
/rust-toolchain.toml     stable
```

`Cargo.toml` adjustments:

```toml
[package]
name = "dobutsu_shogi"
version = "0.1.0"
edition = "2024"
license = "MIT"

[lib]
crate-type = ["cdylib", "rlib"]   # rlib for native tests, cdylib for wasm

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
tsify-next   = { version = "0.5", features = ["js"] }
serde        = { version = "1", features = ["derive"] }
```

The wasm wrapper module is `#[cfg(target_arch = "wasm32")]`-gated so native builds don't pull in `wasm-bindgen`.

---

## 8. CI / deployment

Single workflow at `.github/workflows/deploy.yml`. Triggers on push to `main`.

```
job: build
  - actions/checkout
  - dtolnay/rust-toolchain @ stable, with target wasm32-unknown-unknown
  - actions/cache (cargo registry + ./target + ~/.npm)
  - cargo test --all-targets
  - cargo clippy --all-targets -- -D warnings
  - install wasm-pack (pinned version)
  - wasm-pack build --target web --release --out-dir web/pkg
  - actions/setup-node @ 20
  - npm ci      (in web/)
  - npm run build      (in web/, outputs web/dist/)
  - actions/upload-pages-artifact path=web/dist

job: deploy
  needs: build
  - actions/deploy-pages
```

**One-time manual step**: in repo Settings → Pages → "Build and deployment" → Source: **GitHub Actions**.

**Repo-rename / custom-domain independence.** Vite is configured with `base: './'` (relative URLs). The deployed site works under any path: `lukasz-lew.github.io/DobutsuShogi/`, `lukasz-lew.github.io/Dobutsu/`, or a custom domain — no config change required.

**Toolchain pinning.**

- `rust-toolchain.toml` pins `stable` (channel; ≥ 1.85 for edition 2024).
- `wasm-pack` version pinned in the workflow install step.
- Node 20 LTS, pinned via `actions/setup-node`.

**Linting.** TypeScript: `tsc --strict` only, no ESLint. Rust: `clippy -D warnings` in CI; rustfmt defaults.
