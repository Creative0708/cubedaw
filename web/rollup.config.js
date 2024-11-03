import html from 'rollup-plugin-html2';
import terser from '@rollup/plugin-terser';
import serve from 'rollup-plugin-serve';
import copy from 'rollup-plugin-copy';
// Livereload breaks the audio worklet script so it isn't used
// import livereload from "rollup-plugin-livereload";
import rust from '@wasm-tool/rollup-plugin-rust';
import watchGlobs from 'rollup-plugin-watch-globs';

const isWatch = !!process.env.ROLLUP_WATCH;
const isProd = process.env.NODE_ENV === 'production';

const port = process.env.PORT || 10001;

export default {
    input: {
        cubedaw: 'src/main.js',
        processor: 'src/processor.js'
    },
    output: {
        dir: 'dist',
        format: 'esm',
        preserveModules: true,
    },
    plugins: [
        isWatch && serve({
            contentBase: "dist",
            port,
        }),
        rust({
            debug: !isProd,
            cargoArgs: [
                "-Z",
                "build-std=panic_abort,std",
            ],
            watchPatterns: [
                'crates/*/src/**/*.{rs,js}',
            ]
        }),
        html({
            template: 'index.html',
            entries: {
                cubedaw: {
                    type: 'module'
                }
            },
            minify: isProd && {
                removeComments: true,
                collapseWhitespace: true,
                keepClosingSlash: true,
                minifyCSS: true,
                minifyJS: true,
            },
            exclude: ['processor'],
        }),
        isProd && terser(),
        copy({
            targets: [
                { src: 'assets', dest: 'dist/' },
            ]
        }),
    ],
};