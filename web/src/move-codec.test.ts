import { describe, expect, it } from 'vitest';
import { decodeMove } from './move-codec';

describe('decodeMove', () => {
  it('decodes a slide', () => {
    // from = 7 (row 2 col 1), to = 10 (row 3 col 1)
    expect(decodeMove(0x7a)).toEqual({ kind: 'slide', from: 7, to: 10 });
  });

  it('decodes a chick drop', () => {
    expect(decodeMove(0xc4)).toEqual({ kind: 'drop', piece: 'chick', to: 4 });
  });

  it('decodes an elephant drop', () => {
    expect(decodeMove(0xd0)).toEqual({ kind: 'drop', piece: 'elephant', to: 0 });
  });

  it('decodes a giraffe drop', () => {
    expect(decodeMove(0xeb)).toEqual({ kind: 'drop', piece: 'giraffe', to: 11 });
  });
});
