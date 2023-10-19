import html from 'rollup-plugin-html2';
import terser from '@rollup/plugin-terser';
import copy from 'rollup-plugin-copy';

export default {
    input: 'dist_trunk/cubedaw.js',
    output: {
        dir: 'dist',
        format: 'esm',
        preserveModules: true,
    },
    plugins: [
        html({
            template: 'dist_trunk/index.html',
            minify: {
                removeComments: true,
                collapseWhitespace: true,
                keepClosingSlash: true,
                minifyCSS: true,
                minifyJS: true,
            },
        }),
        terser(),
        copy({
            targets: [
                { src: "dist_trunk/cubedaw_bg.wasm", dest: "dist" },
            ]
        })
    ]
}