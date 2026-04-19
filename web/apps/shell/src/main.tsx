import React from "react";
import ReactDOM from "react-dom/client";

import { bootstrapTheme } from "@ennoia/theme-runtime";
import { App } from "@/App";
import "./styles.css";

bootstrapTheme();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
