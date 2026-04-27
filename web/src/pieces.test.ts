import { describe, expect, it } from 'vitest';
import {
  PIECE_BG_COLORS,
  tilePoints,
  pieceMoves,
  pieceSvg,
  type Piece,
} from './pieces';

describe('tilePoints', () => {
  it('forms the canonical apex-up hexagon for size 30', () => {
    // 6 vertices, clockwise from back-left:
    //   (0, 30), (30, 30), (30, 10), (20, 0), (10, 0), (0, 10)
    // Front corners cut diagonally: each cut is half of a 3×3 grid
    // corner cell (cell side = 10), removing 1/9 of the area total.
    expect(tilePoints(30)).toBe('0,30 30,30 30,10 20,0 10,0 0,10');
  });

  it('scales linearly with size', () => {
    expect(tilePoints(60)).toBe('0,60 60,60 60,20 40,0 20,0 0,20');
  });
});

describe('pieceMoves', () => {
  // Move grid uses (col, row), row 0 = forward (top of cell, owner's POV).

  it('Lion has 8 moves — all 3x3 cells except the centre', () => {
    const moves = pieceMoves('lion');
    expect(moves.length).toBe(8);
    expect(moves.some((d) => d.col === 1 && d.row === 1)).toBe(false);
  });

  it('Giraffe has 4 orthogonal moves', () => {
    const moves = pieceMoves('giraffe');
    expect(new Set(moves.map((d) => `${d.col},${d.row}`))).toEqual(
      new Set(['1,0', '0,1', '2,1', '1,2']),
    );
  });

  it('Elephant has 4 diagonal-corner moves', () => {
    const moves = pieceMoves('elephant');
    expect(new Set(moves.map((d) => `${d.col},${d.row}`))).toEqual(
      new Set(['0,0', '2,0', '0,2', '2,2']),
    );
  });

  it('Chick has 1 move at top-centre (forward)', () => {
    expect(pieceMoves('chick')).toEqual([{ col: 1, row: 0 }]);
  });

  it('Hen has 6 moves: top row + middle sides + bottom centre', () => {
    const moves = pieceMoves('hen');
    expect(new Set(moves.map((d) => `${d.col},${d.row}`))).toEqual(
      new Set(['0,0', '1,0', '2,0', '0,1', '2,1', '1,2']),
    );
  });
});

describe('PIECE_BG_COLORS', () => {
  it('defines a distinct pastel for every piece', () => {
    const values = Object.values(PIECE_BG_COLORS);
    expect(new Set(values).size).toBe(values.length);
    for (const c of values) {
      expect(c).toMatch(/^#[0-9a-f]{6}$/i);
    }
  });
});

describe('pieceSvg', () => {
  const eachPiece: Piece[] = ['lion', 'giraffe', 'elephant', 'chick', 'hen'];

  it.each(eachPiece)('emits a tile polygon for %s', (p) => {
    const svg = pieceSvg(p, 'human', 30);
    expect(svg).toMatch(/<polygon\b[^>]*\bpoints=/);
  });

  it.each(eachPiece)('emits the right number of arrows for %s', (p) => {
    const svg = pieceSvg(p, 'human', 30);
    const arrows = svg.match(/<polygon\b[^>]*\bclass="arrow"/g) ?? [];
    expect(arrows.length).toBe(pieceMoves(p).length);
  });

  it.each(eachPiece)('fills the tile with the pastel bg for %s', (p) => {
    const svg = pieceSvg(p, 'human', 30);
    expect(svg).toContain(`fill="${PIECE_BG_COLORS[p]}"`);
  });

  it.each(eachPiece)('renders arrows in black for %s', (p) => {
    const svg = pieceSvg(p, 'human', 30);
    const arrowMatches = svg.match(/<polygon\b[^>]*\bclass="arrow"[^>]*\bfill="([^"]+)"/g) ?? [];
    expect(arrowMatches.length).toBe(pieceMoves(p).length);
    for (const m of arrowMatches) {
      // Each arrow must use the black ink colour (case-insensitive).
      expect(m.toLowerCase()).toMatch(/fill="#1a1a1a"/);
    }
  });

  it('rotates the entire piece 180° for an AI-owned orientation', () => {
    const svg = pieceSvg('chick', 'ai', 30);
    // The 180° transform (around piece centre) rotates tile + arrows together.
    expect(svg).toMatch(/transform="rotate\(180/);
  });

  it('does not rotate a human-owned piece', () => {
    const svg = pieceSvg('chick', 'human', 30);
    expect(svg).not.toContain('rotate(180');
  });
});
