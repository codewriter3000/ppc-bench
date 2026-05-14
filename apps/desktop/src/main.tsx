/* @refresh reload */
import { render } from "solid-js/web";
import { App } from "./App";
import { ManualsApp } from "./ManualsApp";
import "./styles.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("#root element not found");
}

const isManuals = window.location.hash === "#manuals";
render(() => (isManuals ? <ManualsApp /> : <App />), root);
