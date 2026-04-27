import type { HandCounts, Outcome } from './game';
import { type Owner, type Piece, pieceSvg } from './pieces';

export type Selection =
  | { kind: 'square'; sq: number }
  | { kind: 'hand'; piece: 'chick' | 'elephant' | 'giraffe' };

export interface RenderState {
  board: Uint8Array;
  handHuman: HandCounts;
  handAi: HandCounts;
  humansTurn: boolean;
  thinking: boolean;
  selected: Selection | null;
  legalTargets: Set<number>;
  legalDropTargets: Set<number>;
  lastMove: { from: number | null; to: number } | null;
  outcome: Outcome;
  /** Half-move log; eval is from the player's POV (positive = good for player). */
  moveLog: MoveLogEntry[];
}

export type MoveLogEntry = {
  mover: 'human' | 'ai';
  /** Pre-formatted notation like "Cb2→b3" or "*Eb2". */
  notation: string;
  /** Position eval after this move (player POV); MATE-magnitude for terminal. */
  eval: number;
};

export interface RenderCallbacks {
  onSquareClick: (sq: number) => void;
  onHandClick: (piece: 'chick' | 'elephant' | 'giraffe') => void;
  onNewGame: () => void;
  onUndo: () => void;
  canUndo: boolean;
}

const COLS = 3;
const ROWS = 4;
const CELL_SIZE = 100; // viewBox units; CSS scales to actual pixels
const PIECE_INSET = 6; // padding inside cell

function pieceFromByte(byte: number): { piece: Piece; owner: Owner } | null {
  if (byte === 0) return null;
  const low = byte & 0x0f;
  const owner: Owner = (byte & 0x80) === 0 ? 'human' : 'ai';
  const piece: Piece = (
    [null, 'lion', 'giraffe', 'elephant', 'chick', 'hen'] as const
  )[low] as Piece;
  return { piece, owner };
}

function squareXY(sq: number): { col: number; row: number } {
  const col = sq % COLS;
  const row = Math.floor(sq / COLS);
  // Display: row 0 is human's back rank → physical bottom of the board (high y).
  return { col, row };
}

function cellOriginPx(sq: number): { x: number; y: number } {
  const { col, row } = squareXY(sq);
  // Flip vertically: row 0 (human back rank) goes to the bottom of the screen.
  const visualRow = ROWS - 1 - row;
  return { x: col * CELL_SIZE, y: visualRow * CELL_SIZE };
}

function buildBoardSvg(state: RenderState): string {
  const W = COLS * CELL_SIZE;
  const H = ROWS * CELL_SIZE;
  let inner = '';

  for (let sq = 0; sq < ROWS * COLS; sq++) {
    const { x, y } = cellOriginPx(sq);
    const stone = pieceFromByte(state.board[sq]!);
    const isSelected = state.selected?.kind === 'square' && state.selected.sq === sq;
    const isLastMove = state.lastMove
      ? state.lastMove.from === sq || state.lastMove.to === sq
      : false;
    const isLegalTarget = state.legalTargets.has(sq) || state.legalDropTargets.has(sq);

    const cellClasses = ['cell'];
    if (isSelected) cellClasses.push('selected');

    let cellInner = '';

    cellInner += `<rect class="cell-bg" x="0" y="0" width="${CELL_SIZE}" height="${CELL_SIZE}" fill="var(--bg)" stroke="var(--stroke)" stroke-width="1" />`;

    if (isLastMove) {
      cellInner += `<rect class="last-move" x="0" y="0" width="${CELL_SIZE}" height="${CELL_SIZE}" fill="var(--accent)" opacity="0.15" />`;
    }

    if (isSelected) {
      cellInner += `<rect class="selection" x="2" y="2" width="${CELL_SIZE - 4}" height="${CELL_SIZE - 4}" fill="none" stroke="var(--accent)" stroke-width="3" />`;
    }

    if (stone) {
      const pieceMarkup = pieceSvg(stone.piece, stone.owner, CELL_SIZE - PIECE_INSET * 2);
      cellInner += `<g transform="translate(${PIECE_INSET}, ${PIECE_INSET})">${pieceMarkup}</g>`;
    }

    if (isLegalTarget) {
      if (stone) {
        // Capture indicator: ring around enemy piece.
        cellInner += `<circle class="target-ring" cx="${CELL_SIZE / 2}" cy="${CELL_SIZE / 2}" r="${CELL_SIZE / 2 - 4}" fill="none" stroke="var(--accent)" stroke-width="3" opacity="0.5" />`;
      } else {
        cellInner += `<circle class="target-dot" cx="${CELL_SIZE / 2}" cy="${CELL_SIZE / 2}" r="${CELL_SIZE * 0.12}" fill="var(--accent)" opacity="0.5" />`;
      }
    }

    inner += `<g class="${cellClasses.join(' ')}" data-sq="${sq}" transform="translate(${x}, ${y})">${cellInner}</g>`;
  }

  return `<svg class="board-svg" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">${inner}</svg>`;
}

function buildHandStrip(
  side: 'human' | 'ai',
  hand: HandCounts,
  selected: Selection | null,
): string {
  const owner: Owner = side;
  const pieces: Array<'chick' | 'elephant' | 'giraffe'> = [];
  for (let i = 0; i < hand.chick; i++) pieces.push('chick');
  for (let i = 0; i < hand.elephant; i++) pieces.push('elephant');
  for (let i = 0; i < hand.giraffe; i++) pieces.push('giraffe');

  const HAND_PIECE_SIZE = 48;
  const slots = pieces
    .map((p, i) => {
      const isSelected = side === 'human' && selected?.kind === 'hand' && selected.piece === p && i === pieces.indexOf(p);
      const cls = isSelected ? 'piece-slot selected' : 'piece-slot';
      const interactive = side === 'human' ? `data-hand-piece="${p}"` : '';
      return `<svg class="${cls}" ${interactive} viewBox="0 0 ${HAND_PIECE_SIZE} ${HAND_PIECE_SIZE}" width="${HAND_PIECE_SIZE}" height="${HAND_PIECE_SIZE}" xmlns="http://www.w3.org/2000/svg">${pieceSvg(p, owner, HAND_PIECE_SIZE)}</svg>`;
    })
    .join('');

  return `<div class="hand-strip ${side}"><span class="hand-pieces">${slots}</span></div>`;
}

function fmtEval(n: number): string {
  if (n >= 9000) return '+M';
  if (n <= -9000) return '−M';
  return n >= 0 ? `+${n}` : `−${-n}`;
}

function buildMoveLog(entries: readonly MoveLogEntry[]): string {
  const rows = entries
    .map((e, i) => {
      const ply = i + 1;
      const moverCls = e.mover === 'human' ? 'mv-p' : 'mv-ai';
      const moverLabel = e.mover === 'human' ? 'P' : 'AI';
      return `<div class="mv-row ${moverCls}"><span class="mv-num">${ply}.</span><span class="mv-side">${moverLabel}</span><span class="mv-text">${e.notation}</span><span class="mv-eval">${fmtEval(e.eval)}</span></div>`;
    })
    .join('');
  const header = '<div class="mv-header">Move log</div>';
  const body = entries.length === 0
    ? '<div class="mv-empty">No moves yet.</div>'
    : `<div class="mv-rows">${rows}</div>`;
  return `<div class="move-log">${header}${body}</div>`;
}

function outcomeText(outcome: Outcome, humansTurn: boolean): string {
  // After the last move, humans_turn does not flip on terminal outcomes,
  // so its current value tells us which side just won.
  const winner = humansTurn ? 'You' : 'AI';
  switch (outcome) {
    case 'LionCaptured':
      return `${winner} won — Lion captured`;
    case 'Try':
      return `${winner} won — Try`;
    default:
      return '';
  }
}

export function renderBoard(
  host: HTMLElement,
  state: RenderState,
  cb: RenderCallbacks,
): void {
  const aiHand = buildHandStrip('ai', state.handAi, state.selected);
  const humanHand = buildHandStrip('human', state.handHuman, state.selected);
  const board = buildBoardSvg(state);
  const thinkingOverlay = state.thinking
    ? '<div class="thinking-overlay" aria-label="AI thinking"><span></span><span></span><span></span></div>'
    : '';

  let bannerHtml = '';
  if (state.outcome !== 'Ongoing') {
    bannerHtml = `<div class="banner"><div class="banner-text">${outcomeText(state.outcome, state.humansTurn)}</div><button class="new-game">New game</button></div>`;
  }

  const undoDisabled = cb.canUndo ? '' : 'disabled';
  const controls = `<div class="controls"><button class="new-game">New game</button><button class="undo" ${undoDisabled}>Undo</button></div>`;

  const moveLogHtml = buildMoveLog(state.moveLog);

  host.innerHTML = `${aiHand}<div class="play-area"><div class="board">${board}${thinkingOverlay}</div>${moveLogHtml}</div>${humanHand}${controls}${bannerHtml}`;

  // Wire up event handlers.
  host.querySelectorAll<HTMLElement>('[data-sq]').forEach((el) => {
    const sq = Number(el.dataset['sq']);
    el.addEventListener('click', () => cb.onSquareClick(sq));
  });

  host.querySelectorAll<HTMLElement>('[data-hand-piece]').forEach((el) => {
    const piece = el.dataset['handPiece'] as 'chick' | 'elephant' | 'giraffe';
    el.addEventListener('click', () => cb.onHandClick(piece));
  });

  host.querySelectorAll<HTMLButtonElement>('button.new-game').forEach((b) => {
    b.addEventListener('click', cb.onNewGame);
  });

  const undoBtn = host.querySelector<HTMLButtonElement>('button.undo');
  if (undoBtn) {
    undoBtn.addEventListener('click', cb.onUndo);
  }
}

export function squareIsHumanOwned(byte: number): boolean {
  return byte !== 0 && (byte & 0x80) === 0;
}
