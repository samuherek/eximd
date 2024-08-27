import "./App.css";
import { useState, useEffect } from 'react';
import { assign, setup } from "xstate";
import { useMachine } from "@xstate/react";
import { introMachine } from './Intro';
import { renameMachine } from './Rename';
import Intro from "./Intro";
import Rename from "./Rename";
import { getAnimationDelayStyle } from "./utils";
import clsx from 'clsx';


// ROUTES 
// = Intro
// 1. rename
// 2. duplicate
//
// = RENAME
// 1. collecting files -> loading spinner (we ignore the chanks and wait for all)
// 2. done collected 
//  - see as a tree to toggle
//  - can select files 
//  - can deselect files 
//  - while done collecting, it will take in the exif data in chanks
// 3. rename only after all the exif is collected and the user selects. 
//
//



const appMachine = setup({
    types: {} as {
        context: {
            source: null | string,
        },
        event: { type: "NAV_RENAME", paylaod: string }
        | { type: "NAV_DEDUPLICATE", payload: string }
        | { type: "RESET_TO_INTRO" }
    },
    actions: {
        setSource: assign({
            source: ({ event }) => event.input,
        })
    },
    actors: {
        introMachine,
        renameMachine,
    }
}).createMachine({
    id: "app-machine",
    systemId: "app-machine",
    context: {
        source: null,
    },
    initial: "intro",
    states: {
        intro: {
            invoke: {
                src: 'introMachine',
                id: 'introMachine'
            },
            on: {
                NAV_RENAME: {
                    target: 'rename',
                    actions: assign({
                        source: ({ event }) => event.payload
                    })
                },
                NAV_DEDUPLICATE: {
                    target: 'deduplicate'
                }
            }
        },
        rename: {
            initial: "drop",
            states: {
                drop: {},
                interact: {
                    invoke: {
                        src: 'renameMachine',
                        id: 'renameMachine',
                        input: ({ context }) => ({ source: context.source })
                    },
                }
            },
            on: {
                RESET_TO_INTRO: 'intro'
            }
        },
        deduplicate: {
            on: {
                RESET_TO_INTRO: 'intro'
            }
        }
    }
});

function useNavDelay(delay = 600) {
    const [isLeaving, setIsLeaving] = useState(false);

    function handleNavigate(nav: () => void) {
        setIsLeaving(true);
        setTimeout(() => nav(), delay);
    }

    return [isLeaving, handleNavigate] as const;
}

function Selection({ onRename, onDuplicates }: {
    onRename: () => void,
    onDuplicates: () => void
}) {
    const [isLeaving, navDelay] = useNavDelay(400);

    return (
        <div className="h-full flex flex-col items-center justify-around">
            <div
                className={clsx({
                    "leave-to-top": isLeaving,
                    "enter-from-top": !isLeaving,
                })}
                style={getAnimationDelayStyle(isLeaving ? 5 : 0)}
            >
                <h1 className="py-8 text-4xl font-medium text-center drop-shadow-[0_2px_4px_rgba(0,0,0,0.25)]">
                    Easy Medic for your Media files
                </h1>
            </div>
            <div className={clsx("flex justify-center", {
                "leave-to-down": isLeaving,
                "enter-from-down": !isLeaving,
            })}
                style={getAnimationDelayStyle(isLeaving ? 0 : 3)}
            >
                <button
                    onClick={() => navDelay(onRename)}
                    className="px-24 py-20 group mr-[8vw] translate-y-0 hover:-translate-y-2 shadow-2xl hover:shadow-green-800/10 rounded-[52px] bg-neutral-900 hover:bg-neutral-800 transition-all ease-in-out duration-150">
                    <svg className="w-36 h-36 mb-6 mx-auto group-hover:scale-105 group-hover:text-green-500 transition-all duration-150 easy-in-out" fill="currentColor" xmlns="http://www.w3.org/2000/svg" width="163" height="130" viewBox="0 0 163 130" >
                        <path opacity="0.4" d="M0 20.3125V32.5C0 36.9941 3.64203 40.625 8.15 40.625C12.658 40.625 16.3 36.9941 16.3 32.5V24.375H40.75V105.625H32.6C28.092 105.625 24.45 109.256 24.45 113.75C24.45 118.244 28.092 121.875 32.6 121.875H65.2C69.708 121.875 73.35 118.244 73.35 113.75C73.35 109.256 69.708 105.625 65.2 105.625H57.05V24.375H81.5V32.5C81.5 36.9941 85.142 40.625 89.65 40.625C94.158 40.625 97.8 36.9941 97.8 32.5V20.3125C97.8 13.584 92.3242 8.125 85.575 8.125H48.9H12.225C5.47578 8.125 0 13.584 0 20.3125Z" fill="currentColor" />
                        <path className="text-gray-50 group-hover:text-white" d="M97.8 73.125V77.1875C97.8 81.6816 94.158 85.3125 89.65 85.3125C85.142 85.3125 81.5 81.6816 81.5 77.1875V69.0625C81.5 62.334 86.9758 56.875 93.725 56.875H150.775C157.524 56.875 163 62.334 163 69.0625V77.1875C163 81.6816 159.358 85.3125 154.85 85.3125C150.342 85.3125 146.7 81.6816 146.7 77.1875V73.125H130.4V105.625H134.475C138.983 105.625 142.625 109.256 142.625 113.75C142.625 118.244 138.983 121.875 134.475 121.875H110.025C105.517 121.875 101.875 118.244 101.875 113.75C101.875 109.256 105.517 105.625 110.025 105.625H114.1V73.125H97.8Z" />
                    </svg>
                    <span className="text-xl group-hover:translate-x-0.5">Rename media</span>
                </button>
                <button
                    onClick={() => navDelay(onDuplicates)}
                    className="px-24 py-20 group shadow-2xl translate-y-0 hover:-translate-y-2 hover:shadow-green-800/10 rounded-[52px] bg-neutral-900 hover:bg-neutral-800 transition-all ease-in-out duration-150">
                    <svg className="w-28 h-36 mb-6 mx-auto group-hover:scale-105 group-hover:text-green-500 transition-all duration-150 easy-in-out" fill="currentColor" xmlns="http://www.w3.org/2000/svg" width="150" height="150" viewBox="0 0 150 150">
                        <path opacity="0.4" d="M0 28.125V121.875C0 132.217 8.4082 140.625 18.75 140.625H131.25C141.592 140.625 150 132.217 150 121.875V46.875C150 36.5332 141.592 28.125 131.25 28.125H84.375C81.416 28.125 78.6328 26.748 76.875 24.375L71.25 16.875C67.7051 12.1582 62.1387 9.375 56.25 9.375H18.75C8.4082 9.375 0 17.7832 0 28.125ZM42.1875 79.6875C42.1875 64.1602 54.7852 51.5625 70.3125 51.5625C85.8398 51.5625 98.4375 64.1602 98.4375 79.6875C98.4375 84.9023 97.002 89.7949 94.541 94.0137L105.762 105.234C107.139 106.611 107.812 108.398 107.812 110.215C107.812 112.031 107.139 113.818 105.762 115.195C104.385 116.572 102.598 117.246 100.781 117.246C98.9648 117.246 97.1777 116.572 95.8008 115.195L84.5508 103.945C80.3906 106.406 75.5273 107.812 70.3125 107.812C54.7852 107.812 42.1875 95.2148 42.1875 79.6875ZM56.25 79.6875C56.25 83.4171 57.7316 86.994 60.3688 89.6312C63.006 92.2684 66.5829 93.75 70.3125 93.75C74.0421 93.75 77.619 92.2684 80.2562 89.6312C82.8934 86.994 84.375 83.4171 84.375 79.6875C84.375 75.9579 82.8934 72.381 80.2562 69.7438C77.619 67.1066 74.0421 65.625 70.3125 65.625C66.5829 65.625 63.006 67.1066 60.3688 69.7438C57.7316 72.381 56.25 75.9579 56.25 79.6875Z" fill="currentColor" />
                        <path className="text-gray-50 group-hover:text-white" d="M94.541 94.0137C97.0313 89.8242 98.4375 84.9316 98.4375 79.6875C98.4375 64.1602 85.8398 51.5625 70.3125 51.5625C54.7852 51.5625 42.1875 64.1602 42.1875 79.6875C42.1875 95.2148 54.7852 107.812 70.3125 107.812C75.5273 107.812 80.3906 106.406 84.5801 103.945L95.8008 115.166C98.5547 117.92 103.008 117.92 105.732 115.166C108.457 112.412 108.486 107.959 105.732 105.234L94.5117 94.0137H94.541ZM56.25 79.6875C56.25 75.9579 57.7316 72.381 60.3688 69.7438C63.006 67.1066 66.5829 65.625 70.3125 65.625C74.0421 65.625 77.619 67.1066 80.2562 69.7438C82.8934 72.381 84.375 75.9579 84.375 79.6875C84.375 83.4171 82.8934 86.994 80.2562 89.6312C77.619 92.2684 74.0421 93.75 70.3125 93.75C66.5829 93.75 63.006 92.2684 60.3688 89.6312C57.7316 86.994 56.25 83.4171 56.25 79.6875Z" fill="currentColor" />
                    </svg>
                    <span className="text-xl group-hover:translate-x-0.5">Find duplicates</span>
                </button>
            </div>
            <div
                className={clsx("py-8", {
                    'leave-to-down': isLeaving,
                    "enter-from-down": !isLeaving,
                })}
                style={getAnimationDelayStyle(isLeaving ? 5 : 0)}
            >
                <p className="text-neutral-500">Make sure to learn more about how files are handeled. <span className="text-green-500">Learn more</span></p>
            </div>
        </div>
    )
}

function Drop({ onCancel }: { onCancel: () => void }) {
    const [isLeaving, navDelay] = useNavDelay(400);

    return (
        <div className="h-full flex flex-col items-center justify-around">
            <div
                className={clsx({
                    "leave-to-top": isLeaving,
                    "enter-from-top": !isLeaving,
                })}
                style={getAnimationDelayStyle(isLeaving ? 5 : 0)}
            >
                <h1 className="py-8 text-4xl font-medium text-center drop-shadow-[0_2px_4px_rgba(0,0,0,0.25)]">
                    Rename media files
                </h1>
            </div>
            <div className={clsx("flex justify-center", {
                "leave-to-down": isLeaving,
                "enter-from-down": !isLeaving,
            })}
                style={getAnimationDelayStyle(isLeaving ? 0 : 3)}
            >
                <button
                    className="w-[84vw] py-24 border-4 border-dashed border-green-500 group shadow-2xl translate-y-0 hover:-translate-y-2 hover:shadow-green-800/10 rounded-[52px] bg-neutral-900 hover:bg-neutral-800 transition-all ease-in-out duration-150">
                    <svg className="mb-6 mx-auto w-40 h-40 translate-x-2 scale-100 group-hover:scale-105 transition-all ease-in-out duration-150" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 214 190" >
                        <path opacity="0.4" d="M0 35.625V166.25C0 164.172 0.557292 162.131 1.63472 160.275L43.2458 89.0254C45.3635 85.3516 49.2646 83.125 53.5 83.125H178.333V59.375C178.333 46.2754 167.67 35.625 154.556 35.625H110.901C104.585 35.625 98.5292 33.1387 94.0708 28.6855L84.2253 18.8145C79.767 14.3613 73.7111 11.875 67.3951 11.875H23.7778C10.6628 11.875 0 22.5254 0 35.625ZM0.891667 170.74C1.00312 171 1.11458 171.223 1.22604 171.445C1.3375 171.668 1.44896 171.928 1.59757 172.15C1.3375 171.705 1.11458 171.223 0.891667 170.74ZM171.423 171.111C174.247 168.291 176.364 164.729 177.442 160.758L171.423 171.111Z" />
                        <path d="M53.5 83.125C49.2646 83.125 45.3636 85.3516 43.2458 89.0254L1.63474 160.275C-0.520125 163.949 -0.520125 168.477 1.59758 172.188C3.71529 175.898 7.61633 178.125 11.8889 178.125H160.5C164.735 178.125 168.636 175.898 170.754 172.225L212.365 100.975C214.52 97.3008 214.52 92.7734 212.402 89.0625C210.285 85.3516 206.384 83.125 202.111 83.125H53.5Z" />
                    </svg>
                    <span className="block text-xl mb-1">Drag and drop</span>
                    <span className="text-sm text-neutral-500">(directory or a file)</span>
                </button>
            </div>
            <div className={clsx("py-", {
                'leave-to-down': isLeaving,
                "enter-from-down": !isLeaving,
            })}
                style={getAnimationDelayStyle(isLeaving ? 5 : 0)}
            >
                <button
                    onClick={() => navDelay(onCancel)}
                    className="text-neutral-500 hover:text-green-500 px-4 py-2">
                    Cancel
                </button>
            </div>
        </div >
    )
}

function App() {
    const [state, send] = useMachine(appMachine, {
        inspect: (inspectionEvent) => {
            // type: '@xstate.actor' or
            // type: '@xstate.snapshot' or
            // type: '@xstate.event'
            if (inspectionEvent.type == "@xstate.event") {
                // console.log(inspectionEvent);
            }
        }
    });

    console.log("---------")
    console.log(state);
    // console.log("machine state::: ", state);

    return (
        <div className="h-screen overflow-hidden">
            {state.matches("intro") ? (
                <Selection
                    onRename={() => send({ type: "NAV_RENAME" })}
                    onDuplicates={() => send({ type: "NAV_DEDUPLICATE" })}
                />
            ) : state.matches({ rename: "drop" }) ? (
                <Drop onCancel={() => send({ type: "RESET_TO_INTRO" })} />
            ) : state.matches("rename") ? (
                <Rename actorRef={state.children.renameMachine as any} />
            ) : undefined}
        </div >
    );
}
// <Intro actorRef={state.children.introMachine as any} />

// <svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM369 209L241 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L335 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>

export default App;
