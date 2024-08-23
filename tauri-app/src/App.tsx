import "./App.css";
import { assign, setup } from "xstate";
import { useMachine } from "@xstate/react";
import { introMachine } from './Intro';
import { renameMachine } from './Rename';
import Intro from "./Intro";
import Rename from "./Rename";


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
            invoke: {
                src: 'renameMachine',
                id: 'renameMachine',
                input: ({ context }) => ({ source: context.source })
            }
        },
        deduplicate: {}
    }
});

function App() {
    const [state] = useMachine(appMachine, {
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
        <div className="min-h-screen p-8 bg-neutral-100 dark:bg-neutral-900">
            <div className="grid grid-cols-3 items-center mb-8">
                <div>
                </div>
                <h1 className="text-3xl text-black dark:text-white ">ExiMd</h1>
                <div className="flex justify-end">
                </div>
            </div>
            {state.matches("intro") ? (
                <Intro actorRef={state.children.introMachine as any} />
            ) : state.matches("rename") ? (
                <Rename actorRef={state.children.renameMachine as any} />
            ) : undefined}
        </div >
    );
}

// <svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM369 209L241 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L335 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>

export default App;
