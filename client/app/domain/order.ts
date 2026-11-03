// A playlist's order as the screen changes it. A playlist may name one episode more than once, so its items
// are told apart by their place, and a change is always the whole order sent in place of the one standing
// (`docs/api.md`, 「Playlists」).

/** The order with the item at `from` moved to `to`; the order as it stands where either is outside it. */
export function moved<T>(order: readonly T[], from: number, to: number): T[] {
  const next = [...order];

  if (from < 0 || from >= next.length || to < 0 || to >= next.length || from === to) return next;

  const [item] = next.splice(from, 1);

  next.splice(to, 0, item as T);

  return next;
}

/** The order with the item at `at` taken out. */
export function removedAt<T>(order: readonly T[], at: number): T[] {
  return order.filter((_item, index) => index !== at);
}

/** A key for each item that holds while the order changes: its id and which of its repeats it is. */
export function itemKeys(order: readonly number[]): string[] {
  const seen = new Map<number, number>();

  return order.map((id) => {
    const count = seen.get(id) ?? 0;

    seen.set(id, count + 1);

    return `${id}:${count}`;
  });
}

