import { type Component, For, Show, createResource, createSignal, onMount } from "solid-js";
import { marked } from "marked";
import { DesktopTopbar } from "./DesktopTopbar";
import "./manuals.css";

interface GuideEntry {
  id: string;
  title: string;
  file: string;
}

async function fetchIndex(): Promise<GuideEntry[]> {
  const res = await fetch("/docs/index.json");
  if (!res.ok) throw new Error(`index.json: ${res.status}`);
  return (await res.json()) as GuideEntry[];
}

async function fetchGuide(file: string): Promise<string> {
  const res = await fetch(`/docs/${file}`);
  if (!res.ok) throw new Error(`${file}: ${res.status}`);
  const md = await res.text();
  return marked.parse(md, { async: false }) as string;
}

export const ManualsApp: Component = () => {
  const [index] = createResource(fetchIndex);
  const [activeId, setActiveId] = createSignal<string | null>(null);
  const [html, setHtml] = createSignal<string>("");
  const [error, setError] = createSignal<string | null>(null);

  const select = async (entry: GuideEntry) => {
    setActiveId(entry.id);
    setError(null);
    try {
      setHtml(await fetchGuide(entry.file));
      // Reset scroll on guide switch
      const content = document.querySelector(".manuals-content");
      if (content) content.scrollTop = 0;
    } catch (err) {
      setError(String(err));
      setHtml("");
    }
  };

  onMount(() => {
    document.title = "PPC Manuals";
  });

  // Auto-select first guide when index is loaded.
  const ensureSelected = () => {
    const list = index();
    if (list && list.length > 0 && activeId() === null) {
      void select(list[0]);
    }
  };

  return (
    <div class="manuals-app">
      <DesktopTopbar title="PPC Manuals" />
      <div class="manuals-body">
        <nav class="manuals-sidebar" aria-label="Guide list">
          <div class="manuals-sidebar__title">Reference</div>
          <Show when={index()} fallback={<div class="manuals-sidebar__loading">Loading…</div>}>
            {(list) => {
              ensureSelected();
              return (
                <ul class="manuals-sidebar__list">
                  <For each={list()}>
                    {(entry) => (
                      <li>
                        <button
                          type="button"
                          class="manuals-sidebar__item"
                          classList={{ "manuals-sidebar__item--active": activeId() === entry.id }}
                          onClick={() => void select(entry)}
                        >
                          {entry.title}
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              );
            }}
          </Show>
        </nav>
        <main class="manuals-content">
          <Show when={error()}>
            <div class="manuals-error">{error()}</div>
          </Show>
          {/* eslint-disable-next-line solid/no-innerhtml */}
          <article class="manuals-prose" innerHTML={html()} />
        </main>
      </div>
    </div>
  );
};
