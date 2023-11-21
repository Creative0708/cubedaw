
console.log(String.raw`
   ______      __             __                     ________________________________________
  / ____/_  __/ /_  ___  ____/ /___ __      __      /________________________________________
 / /   / / / / __ \/ _ \/ __  / __ b/ | /| / /     /_________________________________________
/ /___/ /_/ / /_/ /  __/ /_/ / /_/ /| |/ |/ /     /__________________________________________
\____/\__,_/_.___/\___/\__,_/\__,_/ |__/|__/     /___________________________________________

`.replace('b', '`'));


window.onerror = function onerror() {
    console.log("oh no")
};

// addEventListener('error', onerror);

const loadingStatus = document.getElementById('loading-status');
const loadingStatuses = [];

async function loading(msg, promise) {
    loadingStatuses.push(msg);
    loadingStatus.textContent = loadingStatuses.join('\n');
    const val = await promise;
    loadingStatuses.splice(loadingStatuses.indexOf(msg), 1);
    loadingStatus.textContent = loadingStatuses.join('\n');
    return val;
}

import { __META__ } from '../cubedaw_worker/Cargo.toml';

async function init() {
    let runnerPromise = import('../cubedaw/Cargo.toml');
    let workerPromise = fetch(__META__.wasm_bindgen.wasm_path).then((res) => res.arrayBuffer());
    let backendPromise = runnerPromise.then(wasm => wasm.default());

    const warningTimeout = window.setTimeout(() => {
        document.getElementById('loading-warning').style.opacity = 1;
    }, 10000);

    const [runnerWasm, workerData, exports, ..._] = await Promise.all([
        loading('Loading runner code...', runnerPromise),
        loading('Loading worker data...', workerPromise),
        loading('Loading backend...', backendPromise),
        loading('Loading workers...', (async () => {
            window.workerData = await workerPromise;
            const exports = await backendPromise;

            await exports.worker_init();
        })()),
    ]);

    window.clearTimeout(warningTimeout);

    document.getElementById('loading-container').remove();
    document.getElementById('app').removeAttribute('style');

    exports.main();
}

init();
