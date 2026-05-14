import { JSX, type Component, Show } from "solid-js";
import "../styles/panel.css";

export interface PanelProps {
  title: string;
  /** Optional right-aligned header content (counts, toggles, etc.). */
  actions?: JSX.Element;
  bodyStyle?: string;
  children?: JSX.Element;
  /** Fill remaining flex space in the column. Typically set by the parent layout. */
  grow?: boolean;
}

export const Panel: Component<PanelProps> = (props) => {
  return (
    <section
      class={`panel${props.grow ? " panel--grow" : ""}`}
      aria-label={props.title}
    >
      <header class="panel__header">
        <span>{props.title}</span>
        <Show when={props.actions}>
          <span class="panel__header-actions">{props.actions}</span>
        </Show>
      </header>
      <div class="panel__body" style={props.bodyStyle}>
        {props.children}
      </div>
    </section>
  );
};
