export type DropPiece = 'chick' | 'elephant' | 'giraffe';

export type DecodedMove =
  | { kind: 'slide'; from: number; to: number }
  | { kind: 'drop'; piece: DropPiece; to: number };

const DROP_PIECE_BY_FROM: Record<number, DropPiece> = {
  12: 'chick',
  13: 'elephant',
  14: 'giraffe',
};

export function decodeMove(code: number): DecodedMove {
  const to = code & 0x0f;
  const from = code >> 4;
  if (to >= 12) {
    throw new Error(`invalid move code: to=${to} out of range`);
  }
  if (from < 12) {
    return { kind: 'slide', from, to };
  }
  const piece = DROP_PIECE_BY_FROM[from];
  if (!piece) {
    throw new Error(`invalid move code: from=${from} (15 reserved)`);
  }
  return { kind: 'drop', piece, to };
}

export function moveFrom(code: number): number {
  return code >> 4;
}

export function moveTo(code: number): number {
  return code & 0x0f;
}

export function isDropCode(code: number): boolean {
  return (code >> 4) >= 12;
}
