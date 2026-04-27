export type Piece = 'lion' | 'giraffe' | 'elephant' | 'chick' | 'hen';
export type Owner = 'human' | 'ai';

/** Pastel tile-background colours, hue-spread for clear distinctness. */
export const PIECE_BG_COLORS: Record<Piece, string> = {
  lion: '#ecc878',     // warm gold       — hue ~44°
  giraffe: '#ddc190',  // tan             — hue ~35°
  elephant: '#b8cee0', // pale steel blue — hue ~207°
  chick: '#d4e08c',    // pale lime       — hue ~75°
  hen: '#cdb0d8',      // pale lavender   — hue ~285°
};

const ARROW_COLOR = '#1a1a1a';

/**
 * Tile silhouette: a `size × size` square with the two front corners
 * cut diagonally. Each cut removes half of a 3×3 grid corner cell
 * (cell side = `size/3`), so the two cuts together remove 1/9 of the
 * tile area. Apex-up; rotated 180° at render time for the opposite owner.
 */
export function tilePoints(size: number): string {
  const c = size / 3;
  return `0,${size} ${size},${size} ${size},${c} ${size - c},0 ${c},0 0,${c}`;
}

type Cell = { col: 0 | 1 | 2; row: 0 | 1 | 2 };

const MOVES: Record<Piece, readonly Cell[]> = {
  lion: [
    { col: 0, row: 0 }, { col: 1, row: 0 }, { col: 2, row: 0 },
    { col: 0, row: 1 },                      { col: 2, row: 1 },
    { col: 0, row: 2 }, { col: 1, row: 2 }, { col: 2, row: 2 },
  ],
  giraffe: [
                        { col: 1, row: 0 },
    { col: 0, row: 1 },                      { col: 2, row: 1 },
                        { col: 1, row: 2 },
  ],
  elephant: [
    { col: 0, row: 0 },                      { col: 2, row: 0 },
    { col: 0, row: 2 },                      { col: 2, row: 2 },
  ],
  chick: [{ col: 1, row: 0 }],
  hen: [
    { col: 0, row: 0 }, { col: 1, row: 0 }, { col: 2, row: 0 },
    { col: 0, row: 1 },                      { col: 2, row: 1 },
                        { col: 1, row: 2 },
  ],
};

export function pieceMoves(p: Piece): readonly Cell[] {
  return MOVES[p];
}

/**
 * SVG fragment for one piece, sized to fit a `size × size` cell anchored
 * at (0, 0). Move directions render as filled triangle arrows pointing
 * outward from the piece centre. Rotated 180° when owner is `'ai'`.
 */
export function pieceSvg(piece: Piece, owner: Owner, size: number): string {
  const points = tilePoints(size);
  const bg = PIECE_BG_COLORS[piece];

  const tipR = size * 0.32;
  const arrowLen = tipR * 0.6;
  const halfW = tipR * 0.22;
  const cx = size / 2;
  const cy = size / 2;
  const fmt = (n: number) => n.toFixed(3);

  const arrowMarkup = pieceMoves(piece)
    .map((d) => {
      const dx = d.col - 1;
      const dy = d.row - 1;
      const dlen = Math.hypot(dx, dy);
      const ux = dx / dlen;
      const uy = dy / dlen;
      const tx = cx + tipR * ux;
      const ty = cy + tipR * uy;
      const px = -uy;
      const py = ux;
      const bx = tx - arrowLen * ux;
      const by = ty - arrowLen * uy;
      const blX = bx + halfW * px;
      const blY = by + halfW * py;
      const brX = bx - halfW * px;
      const brY = by - halfW * py;
      return `<polygon class="arrow" points="${fmt(tx)},${fmt(ty)} ${fmt(blX)},${fmt(blY)} ${fmt(brX)},${fmt(brY)}" fill="${ARROW_COLOR}" />`;
    })
    .join('');

  const tileMarkup = `<polygon points="${points}" fill="${bg}" stroke="var(--stroke)" stroke-width="1" stroke-linejoin="round" shape-rendering="geometricPrecision" />`;

  const transform = owner === 'ai' ? ` transform="rotate(180 ${size / 2} ${size / 2})"` : '';
  return `<g class="piece-svg" data-piece="${piece}" data-owner="${owner}"${transform}>${tileMarkup}${arrowMarkup}</g>`;
}
