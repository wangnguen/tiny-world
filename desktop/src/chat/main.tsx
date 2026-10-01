import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./chat.css";
import { ChatApp } from "./ChatApp";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ChatApp />
  </StrictMode>,
);
