/**
 * Typed publish/subscribe event bus for inter-panel communication.
 *
 * The bus is the microkernel's IPC primitive: panels and extensions never
 * import each other directly — they publish and subscribe to events.
 */
export type Listener<T> = (payload: T) => void;

export class KernelBus<Events extends Record<string, unknown>> {
  #listeners = new Map<keyof Events, Set<Listener<unknown>>>();

  on<K extends keyof Events>(event: K, listener: Listener<Events[K]>): () => void {
    let set = this.#listeners.get(event);
    if (!set) {
      set = new Set();
      this.#listeners.set(event, set);
    }
    set.add(listener as Listener<unknown>);
    return () => set!.delete(listener as Listener<unknown>);
  }

  emit<K extends keyof Events>(event: K, payload: Events[K]): void {
    const set = this.#listeners.get(event);
    if (!set) return;
    for (const listener of set) {
      try {
        (listener as Listener<Events[K]>)(payload);
      } catch (err) {
        // Isolate listener failures so one bad subscriber can't break the bus.
        console.error(`[KernelBus] listener for "${String(event)}" threw:`, err);
      }
    }
  }

  clear(): void {
    this.#listeners.clear();
  }
}
