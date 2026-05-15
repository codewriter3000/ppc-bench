/* @refresh reload */
import { render } from "solid-js/web";
import { App } from "./App";
import { applyDesktopTheme, loadDesktopSettings, listenForDesktopSettingsUpdates } from "./desktopSettings";
import { ManualsApp } from "./ManualsApp";
import { SettingsApp } from "./SettingsApp";
import "./styles.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("#root element not found");
}
const rootElement = root;

const currentRoute = window.location.hash;

const renderApp = () => {
  if (currentRoute === "#manuals") {
    return <ManualsApp />;
  }
  if (currentRoute === "#settings") {
    return <SettingsApp />;
  }
  return <App />;
};

async function bootstrap() {
  try {
    applyDesktopTheme(await loadDesktopSettings());
  } catch (err) {
    console.warn("Failed to load desktop settings", err);
  }

  render(() => renderApp(), rootElement);

  void listenForDesktopSettingsUpdates((settings) => {
    applyDesktopTheme(settings);
  });
}

void bootstrap();
