import html from 'rollup-plugin-html2';
import terser from '@rollup/plugin-terser';
import serve from 'rollup-plugin-serve';
import livereload from "rollup-plugin-livereload";
import rust from '@wasm-tool/rollup-plugin-rust';

const isWatch = Boolean(process.env.ROLLUP_WATCH);
const isProd = process.env.NODE_ENV === 'production';

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
        }),
        rust({
            debug: !isProd,
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
    ]
}