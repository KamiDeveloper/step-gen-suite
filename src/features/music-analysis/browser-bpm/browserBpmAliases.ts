export function expandTempoAliases(tempo: number): number[] {
  if (isNaN(tempo) || tempo <= 0) return [];
  const list = [tempo / 4, tempo / 2, tempo, tempo * 2, tempo * 4];
  const filtered = list.filter((x) => x >= 40 && x <= 400);
  const rounded = filtered.map((x) => Number(x.toFixed(3)));
  return Array.from(new Set(rounded)).sort((a, b) => a - b);
}
