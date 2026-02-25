import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// remove splash screen after React mounts
const splash = document.getElementById("splash");
if (splash) {
  splash.classList.add("hide");
  setTimeout(() => splash.remove(), 300);
}
