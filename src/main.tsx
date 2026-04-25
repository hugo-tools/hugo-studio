import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { QueryProvider } from "./app/QueryProvider";
import { bootstrapThemeMode } from "./store/theme";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing from index.html");

bootstrapThemeMode();

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <QueryProvider>
      <App />
    </QueryProvider>
  </React.StrictMode>,
);
