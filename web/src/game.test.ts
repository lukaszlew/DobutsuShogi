import { beforeEach, describe, expect, it } from 'vitest';
import {
  GameEngine,
  type AiConfig,
  type AiMove,
  type HandCounts,
  type Outcome,
  type WasmGame,
} from './game';

const TEST_CONFIG: AiConfig = {
  depth: 4,
  chick: 4,
  elephant: 6,
  giraffe: 7,
  hen: 9,
  randomness: 0,
};

class FakeWasmGame implements WasmGame {
  appliedMoves: number[] = [];
  humansTurnFlag: boolean;
  freed = false;
  /** Maps the *count of applied moves at apply-time* to the outcome to return. */
  outcomeAtCount = new Map<number, Outcome>();

  constructor(humanPlaysFirst: boolean) {
    this.humansTurnFlag = humanPlaysFirst;
  }

  legal_moves(): Uint8Array {
    return new Uint8Array([0x01, 0x02, 0x03]);
  }

  apply(mv: number): Outcome {
    this.appliedMoves.push(mv);
    const outcome = this.outcomeAtCount.get(this.appliedMoves.length) ?? 'Ongoing';
    if (outcome === 'Ongoing') {
      this.humansTurnFlag = !this.humansTurnFlag;
    }
    return outcome;
  }

  board(): Uint8Array {
    return new Uint8Array(12);
  }

  hand_human(): HandCounts {
    return { chick: 0, elephant: 0, giraffe: 0 };
  }

  hand_ai(): HandCounts {
    return { chick: 0, elephant: 0, giraffe: 0 };
  }

  ai_move(_config: AiConfig): AiMove | undefined {
    return { mv: 0xff, eval: 0 };
  }

  eval_at_depth(_config: AiConfig): number {
    return 0;
  }

  humans_turn(): boolean {
    return this.humansTurnFlag;
  }

  free(): void {
    this.freed = true;
  }
}

let constructions: { humanFirst: boolean; instance: FakeWasmGame }[];

beforeEach(() => {
  constructions = [];
});

function factory(humanFirst: boolean): FakeWasmGame {
  const inst = new FakeWasmGame(humanFirst);
  constructions.push({ humanFirst, instance: inst });
  return inst;
}

describe('GameEngine — initial state', () => {
  it('starts with no history and no undo', () => {
    const e = new GameEngine(true, factory);
    expect(e.history()).toEqual([]);
    expect(e.canUndo()).toBe(false);
    expect(e.outcomeNow()).toBe('Ongoing');
    expect(e.humansTurn()).toBe(true);
  });

  it('reports humansTurn=false when AI plays first', () => {
    const e = new GameEngine(false, factory);
    expect(e.humansTurn()).toBe(false);
  });
});

describe('GameEngine — apply / undo', () => {
  it('records each applied move in history', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47);
    expect(e.history()).toEqual([0x47]);
    expect(e.humansTurn()).toBe(false);
  });

  it('disables undo while AI is to move', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47); // human moved → AI's turn
    expect(e.canUndo()).toBe(false);
  });

  it('disables undo on history shorter than 2 even on humans_turn', () => {
    // humanPlaysFirst=false, after AI's first move it's human's turn but len=1.
    const e = new GameEngine(false, factory);
    e.applyMove(0x12); // AI moved → human's turn
    expect(e.humansTurn()).toBe(true);
    expect(e.canUndo()).toBe(false);
  });

  it('enables undo after human + AI moves', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47); // human
    e.applyMove(0x12); // ai
    expect(e.canUndo()).toBe(true);
    expect(e.humansTurn()).toBe(true);
  });

  it('undo pops the last 2 moves and rebuilds via the factory', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47);
    e.applyMove(0x12);
    e.applyMove(0x33);
    e.applyMove(0x44);
    e.undo();
    expect(e.history()).toEqual([0x47, 0x12]);
    // Two constructions: the original + the replay rebuild.
    expect(constructions.length).toBe(2);
    expect(constructions[1]!.humanFirst).toBe(true);
    expect(constructions[1]!.instance.appliedMoves).toEqual([0x47, 0x12]);
  });

  it('undo frees the previous wasm instance', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47);
    e.applyMove(0x12);
    e.undo();
    expect(constructions[0]!.instance.freed).toBe(true);
  });

  it('undo restores humansTurn to true', () => {
    const e = new GameEngine(true, factory);
    e.applyMove(0x47);
    e.applyMove(0x12);
    e.applyMove(0x33);
    e.applyMove(0x44);
    e.undo();
    expect(e.humansTurn()).toBe(true);
  });
});

describe('GameEngine — terminal outcomes', () => {
  it('records terminal outcome and disables undo', () => {
    const inst = new FakeWasmGame(true);
    inst.outcomeAtCount.set(2, 'LionCaptured');
    const oneShotFactory = (_: boolean) => inst;
    const e = new GameEngine(true, oneShotFactory);
    e.applyMove(0x47);
    e.applyMove(0x12); // returns LionCaptured
    expect(e.outcomeNow()).toBe('LionCaptured');
    expect(e.canUndo()).toBe(false);
  });

  it('blocks further applyMove calls after terminal outcome', () => {
    const inst = new FakeWasmGame(true);
    inst.outcomeAtCount.set(1, 'Try');
    const oneShotFactory = (_: boolean) => inst;
    const e = new GameEngine(true, oneShotFactory);
    e.applyMove(0x47);
    expect(() => e.applyMove(0x12)).toThrow();
  });
});

describe('GameEngine — pass-throughs', () => {
  it('forwards legalMoves / board / hands / search', () => {
    const e = new GameEngine(true, factory);
    expect(Array.from(e.legalMoves())).toEqual([0x01, 0x02, 0x03]);
    expect(e.board().length).toBe(12);
    expect(e.handHuman()).toEqual({ chick: 0, elephant: 0, giraffe: 0 });
    expect(e.handAi()).toEqual({ chick: 0, elephant: 0, giraffe: 0 });
    expect(e.searchAi(TEST_CONFIG)).toEqual({ mv: 0xff, eval: 0 });
    expect(e.evalAtDepth(TEST_CONFIG)).toBe(0);
  });
});
