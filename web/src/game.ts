export type Outcome = 'Ongoing' | 'LionCaptured' | 'Try';
export type HandCounts = { chick: number; elephant: number; giraffe: number };
export type AiMove = { mv: number; eval: number };

/** Search depth + leaf-eval coefficients + root-jitter magnitude.
 *  Matches the Rust `AiConfig` struct. */
export type AiConfig = {
  depth: number;
  chick: number;
  elephant: number;
  giraffe: number;
  hen: number;
  randomness: number;
};

/**
 * Subset of the wasm-bindgen `Game` class actually used by the JS layer.
 * Keeping the interface narrow makes it trivial to inject a fake in tests.
 */
export interface WasmGame {
  legal_moves(): Uint8Array;
  apply(mv: number): Outcome;
  board(): Uint8Array;
  hand_human(): HandCounts;
  hand_ai(): HandCounts;
  ai_move(config: AiConfig): AiMove | undefined;
  eval_log(config: AiConfig): Int32Array;
  humans_turn(): boolean;
  free(): void;
}

export type WasmGameFactory = (humanPlaysFirst: boolean) => WasmGame;

/**
 * Stateful façade over a wasm `Game`. Tracks the move history so the UI
 * can implement undo by rebuilding the wasm instance and replaying all
 * but the last 2 moves (the AI reply + the human move that triggered it).
 */
export class GameEngine {
  private game: WasmGame;
  private moveHistory: number[] = [];
  private outcome: Outcome = 'Ongoing';

  constructor(
    private readonly humanPlaysFirst: boolean,
    private readonly factory: WasmGameFactory,
  ) {
    this.game = factory(humanPlaysFirst);
  }

  history(): readonly number[] {
    return this.moveHistory;
  }

  outcomeNow(): Outcome {
    return this.outcome;
  }

  humansTurn(): boolean {
    return this.game.humans_turn();
  }

  canUndo(): boolean {
    return (
      this.outcome === 'Ongoing' &&
      this.humansTurn() &&
      this.moveHistory.length >= 2
    );
  }

  applyMove(code: number): Outcome {
    if (this.outcome !== 'Ongoing') {
      throw new Error('cannot apply move: game already over');
    }
    const out = this.game.apply(code);
    this.moveHistory.push(code);
    this.outcome = out;
    return out;
  }

  undo(): void {
    if (this.moveHistory.length < 2) {
      throw new Error('nothing to undo');
    }
    const replay = this.moveHistory.slice(0, this.moveHistory.length - 2);
    this.game.free();
    this.game = this.factory(this.humanPlaysFirst);
    this.moveHistory = [];
    this.outcome = 'Ongoing';
    for (const code of replay) {
      const out = this.game.apply(code);
      this.moveHistory.push(code);
      this.outcome = out;
    }
  }

  legalMoves(): Uint8Array {
    return this.game.legal_moves();
  }

  board(): Uint8Array {
    return this.game.board();
  }

  handHuman(): HandCounts {
    return this.game.hand_human();
  }

  handAi(): HandCounts {
    return this.game.hand_ai();
  }

  searchAi(config: AiConfig): AiMove | undefined {
    return this.game.ai_move(config);
  }

  /** Deterministic evals at depths 1..=10 (hardcoded in wasm), from
   *  the current side-to-move's POV. Returned as a plain number[]. */
  evalLog(config: AiConfig): number[] {
    return Array.from(this.game.eval_log(config));
  }
}
