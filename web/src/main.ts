import init, { Game as WasmGameClass, default_ai_config } from '../pkg/dobutsu_shogi.js';
import { renderBoard, type MoveLogEntry, type RenderState, type Selection } from './board';
import { GameEngine, type AiConfig, type WasmGame } from './game';
import { decodeMove, moveFrom, moveTo } from './move-codec';
import { type Piece, pieceSvg } from './pieces';

function squareCoord(sq: number): string {
  return `${'abc'[sq % 3]}${Math.floor(sq / 3) + 1}`;
}

const PIECE_LETTER_BY_LOW_NIBBLE = ['?', 'L', 'G', 'E', 'C', 'H'];

function pieceLetterFromByte(byte: number): string {
  return PIECE_LETTER_BY_LOW_NIBBLE[byte & 0x7f] ?? '?';
}

function pieceLetterFromName(p: 'chick' | 'elephant' | 'giraffe'): string {
  return p === 'chick' ? 'C' : p === 'elephant' ? 'E' : 'G';
}

function moveNotation(code: number, boardBefore: Uint8Array): string {
  const m = decodeMove(code);
  if (m.kind === 'slide') {
    const piece = pieceLetterFromByte(boardBefore[m.from]!);
    return `${piece}${squareCoord(m.from)}→${squareCoord(m.to)}`;
  }
  return `*${pieceLetterFromName(m.piece)}${squareCoord(m.to)}`;
}

const AI_MIN_THINK_MS = 250;

const FIELD_BOUNDS: Record<keyof AiConfig, [number, number]> = {
  depth: [1, 12],
  randomness: [0, 30],
  chick: [0, 99],
  elephant: [0, 99],
  giraffe: [0, 99],
  hen: [0, 99],
};

const HOST = document.getElementById('app')!;

type DropPiece = 'chick' | 'elephant' | 'giraffe';
const DROP_FROM_BY_PIECE: Record<DropPiece, number> = {
  chick: 12,
  elephant: 13,
  giraffe: 14,
};

const wasmFactory = (humanPlaysFirst: boolean): WasmGame => {
  return new WasmGameClass(humanPlaysFirst) as unknown as WasmGame;
};

let activeRunner: GameRunner | null = null;
let currentConfig: AiConfig | null = null;

async function main(): Promise<void> {
  showLoadingModal();
  try {
    await init();
  } catch (err) {
    console.error('wasm init failed', err);
    showErrorBanner();
    return;
  }
  if (!currentConfig) {
    currentConfig = default_ai_config();
  }
  showConfigModal();
}

function showLoadingModal(): void {
  HOST.innerHTML = `
    <div class="modal-backdrop">
      <div class="modal">
        <h1>Dōbutsu shōgi</h1>
        <div class="button-row">
          <button disabled>Play first</button>
          <button disabled>Play second</button>
        </div>
        <div class="loading-caption">loading…</div>
      </div>
    </div>
  `;
}

type CoefStepper = { piece: Piece; field: 'chick' | 'elephant' | 'giraffe' | 'hen' };

const COEF_STEPPERS: readonly CoefStepper[] = [
  { piece: 'chick', field: 'chick' },
  { piece: 'elephant', field: 'elephant' },
  { piece: 'giraffe', field: 'giraffe' },
  { piece: 'hen', field: 'hen' },
];

const ICON_SIZE = 30;

function stepperRowHtml(iconHtml: string, field: keyof AiConfig, value: number): string {
  return `
    <div class="config-row" data-field="${field}">
      <div class="config-icon">${iconHtml}</div>
      <button class="step-btn step-down" aria-label="decrease">−</button>
      <span class="config-value">${value}</span>
      <button class="step-btn step-up" aria-label="increase">+</button>
    </div>
  `;
}

function showConfigModal(): void {
  const config = currentConfig!;

  const depthRow = stepperRowHtml(
    '<span class="config-text-label">Search depth</span>',
    'depth',
    config.depth,
  );
  const randomnessRow = stepperRowHtml(
    '<span class="config-text-label">Randomness</span>',
    'randomness',
    config.randomness,
  );

  const coefRows = COEF_STEPPERS.map((s) =>
    stepperRowHtml(pieceIconSvg(s.piece), s.field, config[s.field]),
  ).join('');

  const lionRow = `
    <div class="config-row config-row-static">
      <div class="config-icon">${pieceIconSvg('lion')}</div>
      <span class="config-static" aria-label="infinity">∞</span>
    </div>
  `;

  HOST.innerHTML = `
    <div class="modal-backdrop">
      <div class="modal config-modal">
        <h1>Dōbutsu shōgi</h1>
        <div class="config-fieldset">
          <div class="config-fieldset-title">AI settings</div>
          <div class="config-section config-section-depth">${depthRow}${randomnessRow}</div>
          <div class="config-section-label">Piece values</div>
          <div class="config-section config-section-coefs">${coefRows}${lionRow}</div>
        </div>
        <div class="button-row">
          <button class="play-first">Play first</button>
          <button class="play-second">Play second</button>
        </div>
      </div>
    </div>
  `;

  HOST.querySelectorAll<HTMLElement>('.config-row[data-field]').forEach((row) => {
    const field = row.dataset.field as keyof AiConfig;
    const [min, max] = FIELD_BOUNDS[field];
    row.querySelector<HTMLButtonElement>('.step-down')!.addEventListener('click', () => {
      adjustConfig(field, -1, min, max);
    });
    row.querySelector<HTMLButtonElement>('.step-up')!.addEventListener('click', () => {
      adjustConfig(field, +1, min, max);
    });
  });

  HOST.querySelector<HTMLButtonElement>('.play-first')!.addEventListener('click', () => {
    startGame(true);
  });
  HOST.querySelector<HTMLButtonElement>('.play-second')!.addEventListener('click', () => {
    startGame(false);
  });
}

function pieceIconSvg(piece: Piece): string {
  // Stroke disabled at icon scale — diagonal/orthogonal AA mismatch is
  // very visible at 1:1; the pastel fill alone reads well.
  return `<svg viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}" width="${ICON_SIZE}" height="${ICON_SIZE}" aria-label="${piece}">${pieceSvg(piece, 'human', ICON_SIZE, { stroke: false })}</svg>`;
}

function adjustConfig(field: keyof AiConfig, delta: number, min: number, max: number): void {
  const config = currentConfig!;
  const next = Math.max(min, Math.min(max, config[field] + delta));
  if (next === config[field]) return;
  currentConfig = { ...config, [field]: next };
  const row = HOST.querySelector<HTMLElement>(`.config-row[data-field="${field}"]`);
  if (row) row.querySelector<HTMLElement>('.config-value')!.textContent = String(next);
}

function showErrorBanner(): void {
  HOST.innerHTML = `
    <div class="banner">
      <div class="banner-text">Something went wrong</div>
      <button class="reload">New game</button>
    </div>
  `;
  HOST.querySelector<HTMLButtonElement>('.reload')!.addEventListener('click', () => {
    location.reload();
  });
}

function startGame(humanPlaysFirst: boolean): void {
  if (activeRunner) {
    activeRunner.dispose();
  }
  activeRunner = new GameRunner(humanPlaysFirst, currentConfig!);
  activeRunner.kick();
}

class GameRunner {
  private engine: GameEngine;
  private selection: Selection | null = null;
  private lastMove: { from: number | null; to: number } | null = null;
  private thinking = false;
  private disposed = false;
  private moveLog: MoveLogEntry[] = [];

  constructor(humanPlaysFirst: boolean, private readonly config: AiConfig) {
    this.engine = new GameEngine(humanPlaysFirst, wasmFactory);
  }

  dispose(): void {
    this.disposed = true;
  }

  /** Render the current state and trigger an AI move if it's the AI's turn. */
  kick(): void {
    this.render();
    if (!this.engine.humansTurn() && this.engine.outcomeNow() === 'Ongoing') {
      void this.playAiMove();
    }
  }

  private render(): void {
    if (this.disposed) return;
    const board = this.engine.board();
    const handHuman = this.engine.handHuman();
    const handAi = this.engine.handAi();
    const humansTurn = this.engine.humansTurn();
    const outcome = this.engine.outcomeNow();

    const { legalTargets, legalDropTargets } = this.computeLegalTargets();

    const state: RenderState = {
      board,
      handHuman,
      handAi,
      humansTurn,
      thinking: this.thinking,
      selected: this.selection,
      legalTargets,
      legalDropTargets,
      lastMove: this.lastMove,
      outcome,
      moveLog: this.moveLog,
    };

    renderBoard(HOST, state, {
      canUndo: this.engine.canUndo(),
      onSquareClick: (sq) => this.handleSquareClick(sq),
      onHandClick: (piece) => this.handleHandClick(piece),
      onNewGame: () => {
        this.dispose();
        showConfigModal();
      },
      onUndo: () => this.handleUndo(),
    });
  }

  private computeLegalTargets(): { legalTargets: Set<number>; legalDropTargets: Set<number> } {
    const legalTargets = new Set<number>();
    const legalDropTargets = new Set<number>();
    if (!this.selection || !this.engine.humansTurn() || this.engine.outcomeNow() !== 'Ongoing') {
      return { legalTargets, legalDropTargets };
    }
    const legal = this.engine.legalMoves();
    if (this.selection.kind === 'square') {
      for (const code of legal) {
        if (moveFrom(code) === this.selection.sq) {
          legalTargets.add(moveTo(code));
        }
      }
    } else {
      const sentinel = DROP_FROM_BY_PIECE[this.selection.piece];
      for (const code of legal) {
        if (moveFrom(code) === sentinel) {
          legalDropTargets.add(moveTo(code));
        }
      }
    }
    return { legalTargets, legalDropTargets };
  }

  private handleSquareClick(sq: number): void {
    if (!this.engine.humansTurn() || this.engine.outcomeNow() !== 'Ongoing') {
      return;
    }
    const byte = this.engine.board()[sq]!;
    const isHumanPiece = byte !== 0 && (byte & 0x80) === 0;

    if (this.selection) {
      // Clicking the same square deselects.
      if (this.selection.kind === 'square' && this.selection.sq === sq) {
        this.selection = null;
        this.render();
        return;
      }
      // Clicking a legal target performs the move.
      const move = this.findLegalMoveTo(sq);
      if (move !== null) {
        this.applyHumanMove(move);
        return;
      }
      // Clicking another own piece switches selection; otherwise deselect.
      if (isHumanPiece) {
        this.selection = { kind: 'square', sq };
        this.render();
        return;
      }
      this.selection = null;
      this.render();
      return;
    }

    if (isHumanPiece) {
      this.selection = { kind: 'square', sq };
      this.render();
    }
  }

  private handleHandClick(piece: DropPiece): void {
    if (!this.engine.humansTurn() || this.engine.outcomeNow() !== 'Ongoing') {
      return;
    }
    if (this.engine.handHuman()[piece] === 0) return;
    if (this.selection?.kind === 'hand' && this.selection.piece === piece) {
      this.selection = null;
    } else {
      this.selection = { kind: 'hand', piece };
    }
    this.render();
  }

  private findLegalMoveTo(targetSq: number): number | null {
    if (!this.selection) return null;
    const fromSentinel =
      this.selection.kind === 'square'
        ? this.selection.sq
        : DROP_FROM_BY_PIECE[this.selection.piece];
    for (const code of this.engine.legalMoves()) {
      if (moveFrom(code) === fromSentinel && moveTo(code) === targetSq) {
        return code;
      }
    }
    return null;
  }

  private applyHumanMove(code: number): void {
    const decoded = decodeMove(code);
    const boardBefore = this.engine.board().slice();
    this.engine.applyMove(code);
    this.lastMove = {
      from: decoded.kind === 'slide' ? decoded.from : null,
      to: decoded.to,
    };
    this.recordMove(code, boardBefore, 'human');
    this.selection = null;
    this.render();
    if (this.engine.outcomeNow() === 'Ongoing') {
      void this.playAiMove();
    }
  }

  /** Build a MoveLogEntry for a just-applied move and append it to the log.
   *  Evals at depths 1..=10 are post-move and rendered from the player's POV. */
  private recordMove(code: number, boardBefore: Uint8Array, mover: 'human' | 'ai'): void {
    const notation = moveNotation(code, boardBefore);
    let evals: number[];
    if (this.engine.outcomeNow() !== 'Ongoing') {
      // Game ended: the mover delivered the win — fill all depths with ±M.
      const decisive = mover === 'human' ? 10000 : -10000;
      evals = Array(this.config.depth + 2).fill(decisive);
    } else {
      // After the move, side-to-move is the OTHER side. Search returns
      // a score from that side's POV — flip if the mover was the player.
      const sideToMoveEvals = this.engine.evalLog(this.config);
      evals = mover === 'human' ? sideToMoveEvals.map((s) => -s) : sideToMoveEvals;
    }
    this.moveLog.push({ mover, notation, evals });
  }

  private async playAiMove(): Promise<void> {
    this.thinking = true;
    this.render();
    const startedAt = performance.now();
    const ai = this.engine.searchAi(this.config);
    const elapsed = performance.now() - startedAt;
    if (elapsed < AI_MIN_THINK_MS) {
      await sleep(AI_MIN_THINK_MS - elapsed);
    }
    if (this.disposed) return;
    this.thinking = false;
    if (!ai) {
      // No legal moves for AI — defensive; should be unreachable per the spec.
      this.render();
      return;
    }
    const decoded = decodeMove(ai.mv);
    const boardBefore = this.engine.board().slice();
    this.engine.applyMove(ai.mv);
    this.lastMove = {
      from: decoded.kind === 'slide' ? decoded.from : null,
      to: decoded.to,
    };
    this.recordMove(ai.mv, boardBefore, 'ai');
    this.render();
  }

  private handleUndo(): void {
    this.engine.undo();
    this.selection = null;
    this.lastMove = null;
    // engine.undo replays all but the last 2 moves; mirror that on the log.
    this.moveLog = this.moveLog.slice(0, -2);
    this.render();
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

main().catch((err) => {
  console.error('uncaught', err);
  showErrorBanner();
});
