import { For, Show, createResource, createSignal, onMount } from "solid-js";
import { marked } from "marked";
import { DesktopTopbar } from "./DesktopTopbar";
import "./manuals.css";
async function fetchIndex() {
    const res = await fetch("/docs/index.json");
    if (!res.ok)
        throw new Error(`index.json: ${res.status}`);
    return (await res.json());
}
async function fetchGuide(file) {
    const res = await fetch(`/docs/${file}`);
    if (!res.ok)
        throw new Error(`${file}: ${res.status}`);
    const md = await res.text();
    return marked.parse(md, { async: false });
}
export const ManualsApp = () => {
    const [index] = createResource(fetchIndex);
    const [activeId, setActiveId] = createSignal(null);
    const [html, setHtml] = createSignal("");
    const [error, setError] = createSignal(null);
    const select = async (entry) => {
        setActiveId(entry.id);
        setError(null);
        try {
            setHtml(await fetchGuide(entry.file));
            // Reset scroll on guide switch
            const content = document.querySelector(".manuals-content");
            if (content)
                content.scrollTop = 0;
        }
        catch (err) {
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
            const first = list[0];
            if (first)
                void select(first);
        }
    };
    return (<div class="manuals-app">
      <DesktopTopbar title="PPC Manuals"/>
      <div class="manuals-body">
        <nav class="manuals-sidebar" aria-label="Guide list">
          <div class="manuals-sidebar__title">Reference</div>
          <Show when={index()} fallback={<div class="manuals-sidebar__loading">Loading…</div>}>
            {(list) => {
            ensureSelected();
            return (<ul class="manuals-sidebar__list">
                  <For each={list()}>
                    {(entry) => (<li>
                        <button type="button" class="manuals-sidebar__item" classList={{ "manuals-sidebar__item--active": activeId() === entry.id }} onClick={() => void select(entry)}>
                          {entry.title}
                        </button>
                      </li>)}
                  </For>
                </ul>);
        }}
          </Show>
        </nav>
        <main class="manuals-content">
          <Show when={error()}>
            <div class="manuals-error">{error()}</div>
          </Show>
          {/* eslint-disable-next-line solid/no-innerhtml */}
          <article class="manuals-prose" innerHTML={html()}/>
        </main>
      </div>
    </div>);
};
