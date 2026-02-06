import { render } from "solid-js/web";
import Overlay from "./Overlay";
import "./styles/global.css";

render(() => <Overlay />, document.getElementById("root") as HTMLElement);
