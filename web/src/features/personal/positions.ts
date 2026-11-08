/**
 * Where playlist entries sit. Positions are spaced so most moves are one write between two
 * neighbours. When no gap is left a move sends the next neighbour's own position, meaning "just
 * before it"; the server settles that tie and respaces the list (see `settle_position_tie` in
 * backend personal_policy.rs), and does the same when two clients append at once.
 */
export const POSITION_SPACING = 1024;

interface Positioned { position: number }

/** The position after everything already in the list. */
export function appendPosition(rows: readonly Positioned[]): number {
  return rows.reduce((last, row) => Math.max(last, row.position), -POSITION_SPACING) + POSITION_SPACING;
}

/** Positions for `count` entries added one after another at the end. */
export function appendPositions(rows: readonly Positioned[], count: number): number[] {
  const first = appendPosition(rows);
  return Array.from({ length: count }, (_, index) => first + index * POSITION_SPACING);
}

/**
 * The position for the entry at `index` of `ordered`, the list as it should read once the move is
 * done. Its neighbours' positions are the only inputs.
 */
export function positionForMove(ordered: readonly Positioned[], index: number): number {
  const before = ordered[index - 1]?.position;
  const after = ordered[index + 1]?.position;
  if (before === undefined && after === undefined) return 0;
  if (before === undefined) return Math.max(0, after! - POSITION_SPACING);
  if (after === undefined) return before + POSITION_SPACING;
  return after - before >= 2 ? before + Math.floor((after - before) / 2) : after;
}

/** `rows` with the one at `from` moved to `to`. */
export function moved<T>(rows: readonly T[], from: number, to: number): T[] {
  const next = [...rows];
  const [row] = next.splice(from, 1);
  if (row === undefined) return next;
  next.splice(to, 0, row);
  return next;
}
