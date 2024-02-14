
// import wasm from '../crates/cubedaw_worker/Cargo.toml';

// class CubeDAWAudioProcessor extends AudioWorkletProcessor {
//     constructor({ processorOptions: options }) {
//         super();
//         this.id = options.id;

//         this.workers = [];
//         this.idleWorkerPool = [];
//         this.jobQueue = [];

//         this.port.onmessage = (e) => this.onmessage(e.data);

//         this.stopped = true;

//         // Circular buffer for audio
//         this.buffer = new Float32Array(options.circularBufferSize);

//     }

//     onmessage(eventData) {
//         const { type, ...data } = eventData;
//         switch (type) {
//             case 'init':
//                 this.init(data.webassemblyData);
//                 break;
//         }
//     }

//     async init(webassemblyData) {
//         this.wasm = await wasm({ initializeHook: () => { } });

//         this.wasm.default(webassemblyData);

//         console.log(Object.getOwnPropertyNames(this.wasm));

//         this.port.postMessage({ type: 'worker_init' });
//     }

//     /**
//      * @param {Float32Array[][]} outputs
//      */
//     process(inputs, outputs, parameters) {
//         if (this.stopped) {
//             return true;
//         }
//         if (outputs.length != 1) {
//             throw Error(`${outputs.length} outputs sent to CubeDAWAudioProcessor when 1 was expected`);
//         }
//         let output = outputs[0];
//     }
// }

// registerProcessor('cubedaw-processor', CubeDAWAudioProcessor);