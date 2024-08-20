import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { ToastContainer } from "react-toastify";
import 'react-toastify/dist/ReactToastify.css';
import './main.css';

// const _contextClass = {
//     // success: "bg-blue-600",
//     // error: "bg-red-600",
//     // info: "bg-gray-600",
//     // warning: "bg-orange-400",
//     default: "bg-neutral-900 dark:bg-neutral-100",
//     // dark: "bg-white-600 font-gray-300",
// };

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <DndProvider backend={HTML5Backend}>
            <App />
            <ToastContainer position={"bottom-center"}
                className="p-0 bg-transparent"
                toastClassName="p-3 pb-4 flex items-center bg-neutral-100 dark:bg-neutral-900 justify-between rounded-md border border-neutral-200 dark:border-neutral-800"
                bodyClassName="text-sm flex items-center text-neutral-900 dark:text-neutral-100"
                closeButton={false}
            />
        </DndProvider>
    </React.StrictMode>
);
