import "@fontsource-variable/fraunces/opsz.css";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/controls.css";
import "./styles/lists.css";
import { mount } from "svelte";
import App from "./App.svelte";

mount(App, { target: document.body });
