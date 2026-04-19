import { createApp } from "./vendor/vue.esm-browser.prod.js";
import { createViewerApp } from "./viewer_app.js";

createApp(createViewerApp()).mount("#app");
