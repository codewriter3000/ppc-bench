export class SnapshotHistory<T> {
  readonly #capacity: number;
  #entries: T[] = [];
  #cursor = -1;

  constructor(capacity: number) {
    this.#capacity = Math.max(1, capacity | 0);
  }

  push(entry: T): T {
    if (this.#entries.length === this.#capacity) {
      this.#entries.shift();
    }

    this.#entries.push(entry);
    this.#cursor = this.#entries.length - 1;
    return entry;
  }

  back(): T | null {
    if (this.#cursor <= 0) {
      return null;
    }

    this.#cursor -= 1;
    return this.current;
  }

  forward(): T | null {
    if (this.#cursor < 0 || this.#cursor >= this.#entries.length - 1) {
      return null;
    }

    this.#cursor += 1;
    return this.current;
  }

  toLive(): T | null {
    if (!this.#entries.length) {
      return null;
    }

    this.#cursor = this.#entries.length - 1;
    return this.current;
  }

  clear(): void {
    this.#entries = [];
    this.#cursor = -1;
  }

  get current(): T | null {
    if (this.#cursor < 0 || this.#cursor >= this.#entries.length) {
      return null;
    }

    return this.#entries[this.#cursor] ?? null;
  }

  at(index: number): T | null {
    if (index < 0 || index >= this.#entries.length) {
      return null;
    }

    return this.#entries[index] ?? null;
  }

  get length(): number {
    return this.#entries.length;
  }

  get currentIndex(): number {
    return this.#cursor;
  }

  get isAtLive(): boolean {
    return this.#entries.length > 0 && this.#cursor === this.#entries.length - 1;
  }
}