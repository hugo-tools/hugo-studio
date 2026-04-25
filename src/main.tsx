import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { QueryProvider } from "./app/QueryProvider";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing from index.html");

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <QueryProvider>
      <App />
    </QueryProvider>
  </React.StrictMode>,
);
