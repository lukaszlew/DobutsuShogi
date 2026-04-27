import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderBoard, type RenderState, type RenderCallbacks } from './board';
import type { HandCounts, Outcome } from './game';

function emptyHand(): HandCounts {
  return { chick: 0, elephant: 0, giraffe: 0 };
}

function initialBytes(): Uint8Array {
  // Initial position bytes from human's POV.
  // 1 Lion, 2 Giraffe, 3 Elephant, 4 Chick, 5 Hen; high bit = AI.
  const b = new Uint8Array(12);
  // Row 0 (human back rank): E L G
  b[0] = 0x03;
  b[1] = 0x01;
  b[2] = 0x02;
  // Row 1: human chick
  b[4] = 0x04;
  // Row 2: AI chick
  b[7] = 0x84;
  // Row 3 (AI back rank): G L E
  b[9] = 0x82;
  b[10] = 0x81;
  b[11] = 0x83;
  return b;
}

const baseState: RenderState = {
  board: initialBytes(),
  handHuman: emptyHand(),
  handAi: emptyHand(),
  humansTurn: true,
  thinking: false,
  selected: null,
  legalTargets: new Set(),
  legalDropTargets: new Set(),
  lastMove: null,
  outcome: 'Ongoing' as Outcome,
  moveLog: [],
};

let host: HTMLElement;
let callbacks: RenderCallbacks;

beforeEach(() => {
  host = document.createElement('div');
  document.body.appendChild(host);
  callbacks = {
    onSquareClick: vi.fn(),
    onHandClick: vi.fn(),
    onNewGame: vi.fn(),
    onUndo: vi.fn(),
    canUndo: false,
  };
});

describe('renderBoard structure', () => {
  it('renders 12 board cells', () => {
    renderBoard(host, baseState, callbacks);
    expect(host.querySelectorAll('[data-sq]').length).toBe(12);
  });

  it('places pieces on the board from the bytes', () => {
    renderBoard(host, baseState, callbacks);
    const lion = host.querySelector('[data-sq="1"] [data-piece="lion"]');
    expect(lion).not.toBeNull();
    expect(lion?.getAttribute('data-owner')).toBe('human');
    const aiLion = host.querySelector('[data-sq="10"] [data-piece="lion"]');
    expect(aiLion?.getAttribute('data-owner')).toBe('ai');
  });

  it('renders both hand strips', () => {
    renderBoard(host, baseState, callbacks);
    expect(host.querySelector('.hand-strip.human')).not.toBeNull();
    expect(host.querySelector('.hand-strip.ai')).not.toBeNull();
  });

  it('shows the thinking overlay only when the AI is thinking', () => {
    renderBoard(host, baseState, callbacks);
    expect(host.querySelector('.thinking-overlay')).toBeNull();

    renderBoard(host, { ...baseState, thinking: true }, callbacks);
    expect(host.querySelector('.thinking-overlay')).not.toBeNull();
  });

  it('renders New game and Undo controls', () => {
    renderBoard(host, baseState, callbacks);
    expect(host.querySelector('button.new-game')).not.toBeNull();
    expect(host.querySelector('button.undo')).not.toBeNull();
  });

  it('disables undo when callbacks.canUndo is false', () => {
    renderBoard(host, baseState, callbacks);
    const undo = host.querySelector<HTMLButtonElement>('button.undo')!;
    expect(undo.disabled).toBe(true);
  });

  it('enables undo when callbacks.canUndo is true', () => {
    callbacks.canUndo = true;
    renderBoard(host, baseState, callbacks);
    const undo = host.querySelector<HTMLButtonElement>('button.undo')!;
    expect(undo.disabled).toBe(false);
  });
});

describe('renderBoard interactions', () => {
  it('fires onSquareClick when a cell is clicked', () => {
    renderBoard(host, baseState, callbacks);
    const cell = host.querySelector<HTMLElement>('[data-sq="1"]')!;
    cell.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(callbacks.onSquareClick).toHaveBeenCalledWith(1);
  });

  it('fires onUndo when the Undo button is clicked', () => {
    callbacks.canUndo = true;
    renderBoard(host, baseState, callbacks);
    const btn = host.querySelector<HTMLButtonElement>('button.undo')!;
    btn.click();
    expect(callbacks.onUndo).toHaveBeenCalled();
  });

  it('fires onNewGame when the New game button is clicked', () => {
    renderBoard(host, baseState, callbacks);
    const btn = host.querySelector<HTMLButtonElement>('button.new-game')!;
    btn.click();
    expect(callbacks.onNewGame).toHaveBeenCalled();
  });
});

describe('renderBoard highlights', () => {
  it('marks legal target squares with a target indicator', () => {
    const state: RenderState = { ...baseState, legalTargets: new Set([5, 7]) };
    renderBoard(host, state, callbacks);
    expect(host.querySelector('[data-sq="5"] .target-dot')).not.toBeNull();
    // sq 7 has a piece (AI chick) → ring (capture), not dot.
    expect(host.querySelector('[data-sq="7"] .target-ring')).not.toBeNull();
  });

  it('marks the last move on both squares', () => {
    const state: RenderState = { ...baseState, lastMove: { from: 4, to: 7 } };
    renderBoard(host, state, callbacks);
    expect(host.querySelector('[data-sq="4"] .last-move')).not.toBeNull();
    expect(host.querySelector('[data-sq="7"] .last-move')).not.toBeNull();
  });

  it('marks the selected piece', () => {
    const state: RenderState = { ...baseState, selected: { kind: 'square', sq: 4 } };
    renderBoard(host, state, callbacks);
    expect(host.querySelector('[data-sq="4"].selected')).not.toBeNull();
  });
});

describe('renderBoard terminal state', () => {
  it('shows a banner with the outcome and a New game button', () => {
    const state: RenderState = { ...baseState, outcome: 'LionCaptured' as Outcome };
    renderBoard(host, state, callbacks);
    const banner = host.querySelector('.banner');
    expect(banner).not.toBeNull();
    expect(banner!.textContent).toMatch(/Lion captured|won|win/i);
    expect(banner!.querySelector('button.new-game')).not.toBeNull();
  });

  it('does not show a banner when ongoing', () => {
    renderBoard(host, baseState, callbacks);
    expect(host.querySelector('.banner')).toBeNull();
  });
});
