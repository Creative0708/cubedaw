#!/bin/bash

set -e

trunk build

rm -rf rollup/dist_trunk
mv dist_trunk rollup/

cd rollup
npx rollup -c

cd ..
rm -rf dist
cp -r rollup/dist dist