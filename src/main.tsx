import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/tokens.css";
import "./styles/app.css";

const appModule = import.meta.env.MODE === "e2e"
  ? import("./e2e/CookbenchE2EApp")
  : import("./App");

void appModule.then(({ default: App }) => {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});
