import html from 'rollup-plugin-html2';
import terser from '@rollup/plugin-terser';
import copy from 'rollup-plugin-copy';
import path from 'path';
import fs from 'fs';

const TRUNK_DIST_FOLDER = 'dist_trunk/';

const dir = fs.readdirSync(TRUNK_DIST_FOLDER);
const jsEntry = dir.find(file => /cubedaw.*\.js/.test(file));
const wasmEntry = dir.find(file => /cubedaw.*\.wasm/.test(file));

export default {
    input: [
        path.join(TRUNK_DIST_FOLDER, jsEntry),
        path.join(TRUNK_DIST_FOLDER, 'sw.js'),
    ],
    output: {
        dir: 'dist',
        format: 'esm',
        preserveModules: true,
    },
    plugins: [
        html({
            template: path.join(TRUNK_DIST_FOLDER, 'index.html'),
            minify: {
                removeComments: true,
                collapseWhitespace: true,
                keepClosingSlash: true,
                minifyCSS: true,
                minifyJS: true,
            },
            inject: false
        }),
        terser(),
        copy({
            targets: [
                { src: path.join(TRUNK_DIST_FOLDER, wasmEntry), dest: "dist" },
            ]
        })
    ]
}